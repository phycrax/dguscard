#![no_std]

//! Async transport driver for DWIN DGUS T5L displays.
//!
//! Drives a DGUS display over any [`embedded_io_async`] byte stream, using
//! [`dguscard`] as the sans-io frame codec. The split follows `embassy-net`:
//! [`new`] takes caller-provided [`Resources`] storage plus both halves of the
//! transport, and returns a [`Device`] handle owning the **transmitter**
//! alongside a [`Runner`] owning the **receiver**. [`Runner::run`] must be
//! polled continuously — typically from its own task — or no response and no
//! event will ever arrive.
//!
//! Requests go straight to the transport from [`Control`]: there is no queue
//! between the application and the wire. [`Device::write`] and [`Device::read`]
//! transmit inline, then await their correlated reply from the runner.
//!
//! The internal channels use [`NoopRawMutex`], so the handles and the runner
//! must be driven from the same executor (single-threaded); they are `!Send`.
//!
//! Two kinds of traffic flow through the driver:
//!
//! - **Requests** ([`Device::write`] / [`Device::read`]): modbus-style, one in
//!   flight at a time. A write awaits the display's "OK" ACK; a read awaits the
//!   data response correlated by `(addr, wlen)`.
//! - **Events** ([`Device::receive_event`]): data frames the display sends
//!   spontaneously (touch auto-uploads, user input). The runner queues them in
//!   a fixed pool of `E` MTU-sized slots; the application awaits them and
//!   deserializes in place from a [`Content`] borrowing the queue slot. The
//!   slot is released as soon as the closure returns, so decode inside it and
//!   act on the decoded value outside.
//!
//! Use [`Device::split`] to obtain independent [`Control`] and [`Events`]
//! handles when requests and events are driven from separate tasks (e.g. a
//! `select` loop that draws in response to touch events).
//!
//! The two sides fail in disjoint ways and have an error type each: the handles
//! return [`Error`], the receive loop returns [`RunError`]. Neither type can
//! carry a failure belonging to the other side.
//!
//! # Reliability notes
//!
//! - There are no built-in timeouts or retries. Wrap requests in
//!   [`embassy_time::with_timeout`](https://docs.rs/embassy-time) and retry as
//!   your application requires. This is safe: every request carries an internal
//!   sequence number, so a completion belonging to a cancelled (timed-out)
//!   request is discarded by the next request instead of mis-correlating.
//! - Cancelling a request while its frame is still being transmitted leaves a
//!   partial frame on the wire, which the display may need to resynchronize
//!   from. Size timeouts well above the time it takes to shift a frame out.
//! - DGUS has no wire-level sequence numbers. If the display itself answers
//!   *late* — after a request was cancelled and a new one armed — a stale
//!   response can still satisfy a newer request of the same shape. Choose
//!   timeouts generously enough that the display has answered or never will.
//! - A read response is matched by `(addr, wlen)`. A spontaneous frame with the
//!   same address *and* length as an in-flight read is indistinguishable from
//!   its response; avoid reading from variables the display also auto-uploads.
//! - The runner sets a shared `running` flag for as long as [`Runner::run`] is
//!   executing, and every [`Control`] and [`Events`] call checks it on entry,
//!   returning [`Error::NotRunning`] rather than issuing a request nothing will
//!   answer. The flag is checked once, on entry — it does *not* interrupt a
//!   call already awaiting, so a runner that stops mid-request leaves that one
//!   request parked. Timeouts remain mandatory. Note also that the flag tracks
//!   whether `run` is executing, not whether the display is reachable: it is
//!   momentarily clear while the caller backs off between recoverable restarts,
//!   and it never clears for a display that has simply gone quiet.
//! - [`Runner::run`] returns errors instead of hiding them, and the *caller*
//!   picks the recovery policy. Framing noise ([`RunError::Accumulation`]) and a
//!   full event queue ([`RunError::EventQueueFull`]) are recoverable by calling
//!   [`Runner::run`] again in a loop; transport errors mean the byte stream
//!   itself is broken. See [`Runner::run`] for exactly what is lost across a
//!   restart. Size the event queue with the `E` const on [`Resources`].
//!
//! [`NoopRawMutex`]: embassy_sync::blocking_mutex::raw::NoopRawMutex

use core::cell::Cell;
use core::convert::Infallible;
use core::mem::MaybeUninit;

use dguscard::request::{Read, Request, Write};
pub use dguscard::response::Content;
use dguscard::response::{Accumulator, FeedResult, Response};
pub use dguscard::{Address, request, response};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::zerocopy_channel::{Channel, Receiver, Sender};
use embedded_io_async::{Error as _, Read as IoRead, Write as IoWrite};
use serde::{Deserialize, Serialize};

const DEFAULT_MTU: usize = 128;

/// Errors returned by the application-facing handles, [`Control`] and [`Events`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// [`Runner::run`] was not executing when the call was made, so nothing
    /// would have received the reply or queued the event.
    NotRunning,
    /// Request serialization failed.
    Serialize(request::Error),
    /// Transport write error.
    Io(embedded_io_async::ErrorKind),
    /// Response deserialization failed for the given address.
    Deserialize(Address, response::Error),
}

/// Errors that can stop [`Runner::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RunError {
    /// Frame accumulation failed.
    Accumulation(response::Error),
    /// Transport read reached EOF.
    UnexpectedEof,
    /// Transport read error.
    Io(embedded_io_async::ErrorKind),
    /// An unsolicited frame arrived while the event queue was full.
    EventQueueFull,
}

/// Driver configuration, passed to [`new`].
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// Append/verify CRC-16/MODBUS on each frame.
    pub crc: bool,
}

impl Config {
    /// CRC enabled.
    pub const fn new() -> Self {
        Self { crc: true }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// What the in-flight operation is waiting for, armed by [`Control`] just
/// before its frame hits the wire and consumed by the [`Runner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// A write's "OK" ACK.
    Ack,
    /// A read's data response, correlated by address and word length.
    Read { addr: Address, wlen: u8 },
}

/// State shared between the handles and the [`Runner`], living in
/// [`Resources`] rather than in either half so both survive a [`Runner::run`]
/// restart.
struct Shared {
    /// What the in-flight request is waiting for. Armed by [`Control`] just
    /// before the frame starts transmitting — so the reply cannot be observed
    /// before the runner knows to expect it — and cleared by the [`Runner`]
    /// when the matching reply arrives.
    pending: Cell<Option<(Expect, u8)>>,
    /// Whether [`Runner::run`] is currently executing. Held by a [`RunGuard`]
    /// living in the `run` future, and read by every [`Control`] and [`Events`]
    /// call.
    running: Cell<bool>,
}

impl Shared {
    const fn new() -> Self {
        Self {
            pending: Cell::new(None),
            running: Cell::new(false),
        }
    }
}

/// RAII guard tying [`Shared::running`] to a [`Runner::run`] call: constructed
/// as `run` starts, dropped whenever it leaves — by returning an error, by the
/// task exiting, or by the future being cancelled.
struct RunGuard<'d> {
    shared: &'d Shared,
}

impl<'d> RunGuard<'d> {
    fn new(shared: &'d Shared) -> Self {
        shared.running.set(true);
        Self { shared }
    }
}

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        self.shared.running.set(false);
    }
}

/// A completed operation's result (runner → handle).
struct ResponseFrame<const MTU: usize> {
    seq: u8,
    len: usize,
    buf: [u8; MTU],
}

impl<const MTU: usize> ResponseFrame<MTU> {
    const INIT: Self = Self {
        seq: 0,
        len: 0,
        buf: [0; MTU],
    };
}

/// A queued unsolicited data frame (runner → application).
struct EventFrame<const MTU: usize> {
    addr: Address,
    wlen: u8,
    len: usize,
    buf: [u8; MTU],
}

impl<const MTU: usize> EventFrame<MTU> {
    const INIT: Self = Self {
        addr: Address::Word(0),
        wlen: 0,
        len: 0,
        buf: [0; MTU],
    };
}

/// State shared between [`Control`] and [`Runner`]: the two zero-copy channels,
/// borrowing the buffer arrays in [`Resources`], plus the flags in [`Shared`].
struct StateInner<'a, const MTU: usize> {
    resp: Channel<'a, NoopRawMutex, ResponseFrame<MTU>>,
    events: Channel<'a, NoopRawMutex, EventFrame<MTU>>,
    shared: Shared,
}

/// Caller-provided storage for the driver, like `embassy_net::StackResources`.
///
/// `MTU` bounds the largest sendable/receivable frame. `E` is the depth
/// of the unsolicited event queue — size it for the worst-case burst of
/// touch events between two [`Events::receive`] calls.
pub struct Resources<const MTU: usize = DEFAULT_MTU, const E: usize = 4> {
    resp_bufs: [ResponseFrame<MTU>; 1],
    event_bufs: [EventFrame<MTU>; E],
    inner: MaybeUninit<StateInner<'static, MTU>>,
}

impl<const MTU: usize, const E: usize> Resources<MTU, E> {
    /// Creates new driver storage.
    pub const fn new() -> Self {
        Self {
            resp_bufs: [ResponseFrame::INIT; 1],
            event_bufs: [EventFrame::INIT; E],
            inner: MaybeUninit::uninit(),
        }
    }
}

impl<const MTU: usize, const E: usize> Default for Resources<MTU, E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Application-facing handle bundling [`Control`] and [`Events`].
///
/// Convenient for simple sequential use. Call [`Device::split`] when requests
/// and events must be driven from separate tasks.
pub struct Device<'d, TX: IoWrite, const MTU: usize = DEFAULT_MTU> {
    control: Control<'d, TX, MTU>,
    events: Events<'d, MTU>,
}

/// Request half of the handle: owns the transmitter, serializes requests
/// straight onto it, and awaits their correlated completion from the
/// [`Runner`].
pub struct Control<'d, TX: IoWrite, const MTU: usize = DEFAULT_MTU> {
    tx: TX,
    /// Serialization scratch for the request being sent. Only one request is
    /// in flight at a time (enforced by `&mut self`), so one buffer suffices.
    buf: [u8; MTU],
    resp_receiver: Receiver<'d, NoopRawMutex, ResponseFrame<MTU>>,
    shared: &'d Shared,
    seq: u8,
    crc: bool,
}

/// Event half of the handle: receives unsolicited data frames queued by the
/// [`Runner`].
pub struct Events<'d, const MTU: usize = DEFAULT_MTU> {
    event_receiver: Receiver<'d, NoopRawMutex, EventFrame<MTU>>,
    shared: &'d Shared,
}

/// RX worker. [`Runner::run`] must be polled continuously for request
/// completions and event delivery to make progress.
pub struct Runner<'d, RX: IoRead, const MTU: usize = DEFAULT_MTU> {
    rx: RX,
    resp_sender: Sender<'d, NoopRawMutex, ResponseFrame<MTU>>,
    event_sender: Sender<'d, NoopRawMutex, EventFrame<MTU>>,
    shared: &'d Shared,
    crc: bool,
}

/// Creates a driver instance from a split transport.
///
/// Returns the [`Device`] handle, which owns `tx`, and the [`Runner`], which
/// owns `rx`.
pub fn new<'d, RX, TX, const MTU: usize, const E: usize>(
    resources: &'d mut Resources<MTU, E>,
    rx: RX,
    tx: TX,
    config: Config,
) -> (Device<'d, TX, MTU>, Runner<'d, RX, MTU>)
where
    RX: IoRead,
    TX: IoWrite,
{
    let crc = config.crc;

    // Self-referential init, mirroring `embassy-net-driver-channel`: the
    // channels in `StateInner` borrow `resources.resp_bufs` /
    // `resources.event_bufs`, which are fields of the same `resources`.
    //
    // SAFETY: the borrow laundered from `'static` to `'d` is sound because the
    // returned handles carry `'d`, so `resources` cannot move or be reused
    // while any of the internal references are live, and those references are
    // never touched once the `'d` borrow ends.
    let inner_uninit: *mut MaybeUninit<StateInner<'d, MTU>> =
        (&mut resources.inner as *mut MaybeUninit<StateInner<'static, MTU>>).cast();
    let inner = unsafe { &mut *inner_uninit }.write(StateInner {
        resp: Channel::new(&mut resources.resp_bufs[..]),
        events: Channel::new(&mut resources.event_bufs[..]),
        shared: Shared::new(),
    });

    let (resp_sender, resp_receiver) = inner.resp.split();
    let (event_sender, event_receiver) = inner.events.split();
    let shared = &inner.shared;

    let device = Device {
        control: Control {
            tx,
            buf: [0; MTU],
            resp_receiver,
            shared,
            seq: 0,
            crc,
        },
        events: Events { event_receiver, shared },
    };
    let runner = Runner {
        rx,
        resp_sender,
        event_sender,
        shared,
        crc,
    };
    (device, runner)
}

impl<'d, TX: IoWrite, const MTU: usize> Device<'d, TX, MTU> {
    /// Writes `val` to `addr` and awaits the ACK. See [`Control::write`].
    pub async fn write<T: Serialize + ?Sized>(&mut self, addr: Address, val: &T) -> Result<(), Error> {
        self.control.write(addr, val).await
    }

    /// Reads `wlen` words from `addr` and deserializes `T`. See
    /// [`Control::read`].
    pub async fn read<T: for<'a> Deserialize<'a>>(&mut self, addr: Address, wlen: u8) -> Result<T, Error> {
        self.control.read(addr, wlen).await
    }

    /// Waits for the next unsolicited data frame. See [`Events::receive`].
    pub async fn receive_event<R>(&mut self, f: impl FnOnce(Content<'_>) -> R) -> Result<R, Error> {
        self.events.receive(f).await
    }

    /// Returns the next queued unsolicited frame without waiting. See
    /// [`Events::try_receive`].
    pub fn try_receive_event<R>(&mut self, f: impl FnOnce(Content<'_>) -> R) -> Result<Option<R>, Error> {
        self.events.try_receive(f)
    }

    /// Splits the handle into its independent request and event halves.
    pub fn split(self) -> (Control<'d, TX, MTU>, Events<'d, MTU>) {
        (self.control, self.events)
    }
}

impl<'d, TX: IoWrite, const MTU: usize> Control<'d, TX, MTU> {
    /// Discards completions left over from cancelled (timed-out) requests, then
    /// stamps and returns the sequence number for a new request.
    fn begin_request(&mut self) -> u8 {
        while self.resp_receiver.try_receive().is_some() {
            self.resp_receiver.receive_done();
        }
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    /// Transmits one request frame built by `build` and awaits its correlated
    /// completion, mapping the completion payload with `complete`.
    async fn transfer<R>(
        &mut self,
        expect: Expect,
        build: impl FnOnce(&mut [u8], bool) -> Result<usize, request::Error>,
        complete: impl FnOnce(&[u8]) -> Result<R, Error>,
    ) -> Result<R, Error> {
        // Nothing is receiving, so this frame could only ever time out. Fail
        // before it reaches the wire.
        if !self.shared.running.get() {
            return Err(Error::NotRunning);
        }

        let seq = self.begin_request();
        let len = build(&mut self.buf, self.crc).map_err(Error::Serialize)?;

        // Arm correlation before the frame hits the wire, so the reply cannot
        // be observed before the runner knows to expect it.
        self.shared.pending.set(Some((expect, seq)));
        if let Err(e) = self.tx.write_all(&self.buf[..len]).await {
            self.shared.pending.set(None);
            return Err(Error::Io(e.kind()));
        }

        loop {
            let resp = self.resp_receiver.receive().await;
            if resp.seq == seq {
                let out = complete(&resp.buf[..resp.len]);
                self.resp_receiver.receive_done();
                return out;
            }
            self.resp_receiver.receive_done();
        }
    }

    /// Writes `val` to the variable (word) address `addr` and awaits the ACK.
    ///
    /// Only one request may be in flight at a time (enforced by `&mut self`).
    /// Cancel-safe with one caveat: a completion arriving after this future is
    /// dropped is discarded by the next request, but cancelling *during*
    /// transmission leaves a partial frame on the wire.
    ///
    /// # Errors
    ///
    /// [`Error::NotRunning`] if [`Runner::run`] was not executing on entry, in
    /// which case nothing is transmitted. Otherwise [`Error::Serialize`] or
    /// [`Error::Io`].
    pub async fn write<T: Serialize + ?Sized>(&mut self, addr: Address, val: &T) -> Result<(), Error> {
        self.transfer(
            Expect::Ack,
            |buf, crc| {
                let mut req = Request::with_slice(buf, addr, Write)?;
                req.push(val)?;
                Ok(req.finalize(crc)?.len())
            },
            |_| Ok(()),
        )
        .await
    }

    /// Reads `wlen` words from `addr`, awaits the response, and deserializes `T`.
    ///
    /// Only one request may be in flight at a time (enforced by `&mut self`).
    /// Cancel-safe with one caveat: a completion arriving after this future is
    /// dropped is discarded by the next request, but cancelling *during*
    /// transmission leaves a partial frame on the wire.
    ///
    /// # Errors
    ///
    /// [`Error::NotRunning`] if [`Runner::run`] was not executing on entry, in
    /// which case nothing is transmitted. Otherwise [`Error::Serialize`],
    /// [`Error::Io`], or [`Error::Deserialize`].
    pub async fn read<T: for<'a> Deserialize<'a>>(&mut self, addr: Address, wlen: u8) -> Result<T, Error> {
        self.transfer(
            Expect::Read { addr, wlen },
            |buf, crc| Ok(Request::with_slice(buf, addr, Read(wlen))?.finalize(crc)?.len()),
            |bytes| {
                Content::new(addr, wlen, bytes)
                    .take::<T>()
                    .map_err(|e| Error::Deserialize(addr, e))
            },
        )
        .await
    }
}

impl<'d, const MTU: usize> Events<'d, MTU> {
    /// Waits for the next unsolicited data frame and passes it to `f` as a
    /// [`Content`] borrowing the queue slot.
    ///
    /// Deserialize with [`Content::take`] (repeatedly for concatenated values)
    /// and return whatever the application needs; the slot is released as soon
    /// as `f` returns, so `f` cannot hold on to the borrowed bytes. Requests
    /// that depend on an event are therefore issued *after* `receive` returns,
    /// on the value `f` produced.
    ///
    /// Cancel-safe: dropping the future before it completes consumes nothing
    /// and never calls `f`.
    ///
    /// # Errors
    ///
    /// [`Error::NotRunning`] if [`Runner::run`] was not executing on entry —
    /// nothing would ever queue an event to wait for. The flag is checked
    /// *once*, on entry: if the runner stops while this future is already
    /// parked, it stays parked. Wrap the call in a timeout.
    pub async fn receive<R>(&mut self, f: impl FnOnce(Content<'_>) -> R) -> Result<R, Error> {
        if !self.shared.running.get() {
            return Err(Error::NotRunning);
        }

        let out = {
            let slot = self.event_receiver.receive().await;
            f(Content::new(slot.addr, slot.wlen, &slot.buf[..slot.len]))
        };
        self.event_receiver.receive_done();
        Ok(out)
    }

    /// Returns `f`'s result for the next queued unsolicited frame, or
    /// `Ok(None)` if the queue is empty. See [`Events::receive`].
    ///
    /// # Errors
    ///
    /// [`Error::NotRunning`] if [`Runner::run`] was not executing on entry. The
    /// error takes precedence over the queue, so frames left over from before
    /// the runner stopped are not drained by this call.
    pub fn try_receive<R>(&mut self, f: impl FnOnce(Content<'_>) -> R) -> Result<Option<R>, Error> {
        if !self.shared.running.get() {
            return Err(Error::NotRunning);
        }

        let out = {
            let Some(slot) = self.event_receiver.try_receive() else {
                return Ok(None);
            };
            f(Content::new(slot.addr, slot.wlen, &slot.buf[..slot.len]))
        };
        self.event_receiver.receive_done();
        Ok(Some(out))
    }
}

impl<'d, RX: IoRead, const MTU: usize> Runner<'d, RX, MTU> {
    /// Drives the receive loop: accumulates frames off the transport, hands
    /// those completing an in-flight request back to [`Control`], and queues
    /// every other data frame for [`Events`].
    ///
    /// `rx_buf` is the caller-provided scratch buffer for incoming transport
    /// reads; the caller owns its storage (e.g. a `static` or stack slot)
    /// instead of it being embedded in this future's state.
    ///
    /// # Errors and restarts
    ///
    /// Returns on the first error instead of hiding it; the caller picks the
    /// recovery policy by matching on the error and, for recoverable ones,
    /// calling `run` again in a loop:
    ///
    /// - [`RunError::Accumulation`]: framing noise (stray bytes, a partial frame
    ///   at startup, a CRC mismatch). Restarting resynchronizes on the next
    ///   frame header. The partially accumulated frame and the unprocessed
    ///   remainder of the current read window are lost; a request whose
    ///   response was in that data hangs until the caller's timeout and can
    ///   then be retried.
    /// - [`RunError::EventQueueFull`]: an unsolicited frame arrived while the
    ///   event queue was full. That frame is lost; restarting resumes normal
    ///   operation. Size the queue with the `E` const on [`Resources`].
    /// - [`RunError::Io`] / [`RunError::UnexpectedEof`]: the transport itself
    ///   failed; restart only after the underlying stream has been repaired.
    ///
    /// Transmission is unaffected by any of this — [`Control`] owns the
    /// transmitter — so a restart never truncates an outgoing frame. Request
    /// correlation lives in shared state rather than in this future, so it
    /// survives a restart untouched.
    ///
    /// # Run state
    ///
    /// While this call is executing it sets a shared `running` flag; the flag
    /// clears on every exit path — an error return, the task exiting, or this
    /// future being cancelled — and [`Control`] and [`Events`] calls made while
    /// it is clear fail with [`Error::NotRunning`] instead of parking forever.
    ///
    /// A restart loop that calls `run` again without awaiting in between never
    /// exposes the gap, since the handles cannot be polled in it. Awaiting a
    /// backoff before restarting does expose it, and requests issued during
    /// that window fail — which is usually what you want, but it means
    /// [`Error::NotRunning`] reports "the receive loop is not running", not "the
    /// display is unreachable".
    pub async fn run(&mut self, rx_buf: &mut [u8]) -> Result<Infallible, RunError> {
        let mut acc = Accumulator::<MTU>::new(self.crc);
        // Publishes `running` for exactly as long as this call runs; its `Drop`
        // clears it on every exit path, cancellation included.
        let _guard = RunGuard::new(self.shared);
        loop {
            let n = self.rx.read(rx_buf).await.map_err(|e| RunError::Io(e.kind()))?;
            if n == 0 {
                return Err(RunError::UnexpectedEof);
            }

            let mut window: &[u8] = &rx_buf[..n];
            while !window.is_empty() {
                window = match acc.feed(window) {
                    FeedResult::Consumed => break,
                    FeedResult::Error(e, _) => return Err(RunError::Accumulation(e)),
                    FeedResult::Success(response, rest) => {
                        match response {
                            Response::Ack => {
                                if let Some((Expect::Ack, seq)) = self.shared.pending.get() {
                                    self.shared.pending.set(None);
                                    let slot = self.resp_sender.send().await;
                                    slot.seq = seq;
                                    slot.len = 0;
                                    self.resp_sender.send_done();
                                }
                                // Unmatched ACKs belong to cancelled requests;
                                // drop them.
                            }
                            Response::Data(content) => {
                                if let Some((Expect::Read { addr: a, wlen: w }, seq)) = self.shared.pending.get()
                                    && a == content.addr()
                                    && w == content.wlen()
                                {
                                    self.shared.pending.set(None);
                                    let bytes = content.as_bytes();
                                    let slot = self.resp_sender.send().await;
                                    slot.buf[..bytes.len()].copy_from_slice(bytes);
                                    slot.len = bytes.len();
                                    slot.seq = seq;
                                    self.resp_sender.send_done();
                                } else {
                                    // Unsolicited (including a data frame
                                    // arriving while a write awaits its ACK):
                                    // queue for `Events`.
                                    let (addr, wlen) = (content.addr(), content.wlen());
                                    let bytes = content.as_bytes();
                                    let Some(slot) = self.event_sender.try_send() else {
                                        return Err(RunError::EventQueueFull);
                                    };
                                    slot.addr = addr;
                                    slot.wlen = wlen;
                                    slot.buf[..bytes.len()].copy_from_slice(bytes);
                                    slot.len = bytes.len();
                                    self.event_sender.send_done();
                                }
                            }
                        }
                        rest
                    }
                };
            }
        }
    }
}
