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
//! - **Events** ([`Device::event`]): data frames the display sends
//!   spontaneously (touch auto-uploads, user input). The runner queues them in
//!   a fixed pool of `E` MTU-sized slots; the application awaits them and
//!   deserializes in place via the zero-copy [`Event`] guard.
//!
//! Use [`Device::split`] to obtain independent [`Control`] and [`Events`]
//! handles when one task must hold an [`Event`] borrow while issuing requests
//! (e.g. a `select` loop that draws in response to touch events).
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
//! - Neither half signals the other's death. If the runner's task stops, the
//!   handles simply never complete; a timeout is the only guard.
//! - [`Runner::run`] returns errors instead of hiding them, and the *caller*
//!   picks the recovery policy. Framing noise ([`Error::Accumulation`]) and a
//!   full event queue ([`Error::EventQueueFull`]) are recoverable by calling
//!   [`Runner::run`] again in a loop; transport errors mean the byte stream
//!   itself is broken. See [`Runner::run`] for exactly what is lost across a
//!   restart. Size the event queue with the `E` const on [`Resources`].
//!
//! [`NoopRawMutex`]: embassy_sync::blocking_mutex::raw::NoopRawMutex

use core::{cell::Cell, convert::Infallible, mem::MaybeUninit};

pub use dguscard::Address;

use dguscard::{
    request::{Read, Request, Write},
    response::{Accumulator, Content, FeedResult, Response},
};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    zerocopy_channel::{Channel, Receiver, Sender},
};
use embedded_io_async::{Error as _, Read as IoRead, Write as IoWrite};
use serde::{Deserialize, Serialize};

const DEFAULT_MTU: usize = 128;

/// Driver errors.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Frame accumulation failed.
    Accumulation(dguscard::Error),
    /// Request serialization failed.
    Serialize(dguscard::Error),
    /// Response deserialization failed for the given address.
    Deserialize(Address, dguscard::Error),
    /// Transport read reached EOF.
    UnexpectedEof,
    /// Transport read error.
    Read(embedded_io_async::ErrorKind),
    /// Transport write error.
    Write(embedded_io_async::ErrorKind),
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
/// borrowing the buffer arrays in [`Resources`], plus the request correlation
/// slot.
struct StateInner<'a, const MTU: usize> {
    resp: Channel<'a, NoopRawMutex, ResponseFrame<MTU>>,
    events: Channel<'a, NoopRawMutex, EventFrame<MTU>>,
    /// What the in-flight request is waiting for. Armed by [`Control`] just
    /// before the frame starts transmitting — so the reply cannot be observed
    /// before the runner knows to expect it — and cleared by the [`Runner`]
    /// when the matching reply arrives. Living here rather than in either half
    /// keeps correlation intact across a [`Runner::run`] restart.
    pending: Cell<Option<(Expect, u8)>>,
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
/// Convenient for simple sequential use; note that [`Device::event`] borrows
/// the whole handle, so a held [`Event`] blocks [`Device::write`] /
/// [`Device::read`]. Call [`Device::split`] when one task needs to issue
/// requests while an event is being processed.
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
    pending: &'d Cell<Option<(Expect, u8)>>,
    seq: u8,
    crc: bool,
}

/// Event half of the handle: receives unsolicited data frames queued by the
/// [`Runner`].
pub struct Events<'d, const MTU: usize = DEFAULT_MTU> {
    event_receiver: Receiver<'d, NoopRawMutex, EventFrame<MTU>>,
}

/// RX worker. [`Runner::run`] must be polled continuously for request
/// completions and event delivery to make progress.
pub struct Runner<'d, RX: IoRead, const MTU: usize = DEFAULT_MTU> {
    rx: RX,
    resp_sender: Sender<'d, NoopRawMutex, ResponseFrame<MTU>>,
    event_sender: Sender<'d, NoopRawMutex, EventFrame<MTU>>,
    pending: &'d Cell<Option<(Expect, u8)>>,
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
        pending: Cell::new(None),
    });

    let (resp_sender, resp_receiver) = inner.resp.split();
    let (event_sender, event_receiver) = inner.events.split();
    let pending = &inner.pending;

    let device = Device {
        control: Control {
            tx,
            buf: [0; MTU],
            resp_receiver,
            pending,
            seq: 0,
            crc,
        },
        events: Events { event_receiver },
    };
    let runner = Runner {
        rx,
        resp_sender,
        event_sender,
        pending,
        crc,
    };
    (device, runner)
}

impl<'d, TX: IoWrite, const MTU: usize> Device<'d, TX, MTU> {
    /// Writes `val` to `addr` and awaits the ACK. See [`Control::write`].
    pub async fn write<T: Serialize + ?Sized>(
        &mut self,
        addr: Address,
        val: &T,
    ) -> Result<(), Error> {
        self.control.write(addr, val).await
    }

    /// Reads `wlen` words from `addr` and deserializes `T`. See
    /// [`Control::read`].
    pub async fn read<T: for<'a> Deserialize<'a>>(
        &mut self,
        addr: Address,
        wlen: u8,
    ) -> Result<T, Error> {
        self.control.read(addr, wlen).await
    }

    /// Waits for the next unsolicited data frame. See [`Events::receive`].
    pub async fn event(&mut self) -> Event<'_, 'd, MTU> {
        self.events.receive().await
    }

    /// Returns the next queued unsolicited frame without waiting. See
    /// [`Events::try_receive`].
    pub fn try_event(&mut self) -> Option<Event<'_, 'd, MTU>> {
        self.events.try_receive()
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
        build: impl FnOnce(&mut [u8], bool) -> Result<usize, dguscard::Error>,
        complete: impl FnOnce(&[u8]) -> Result<R, Error>,
    ) -> Result<R, Error> {
        let seq = self.begin_request();
        let len = build(&mut self.buf, self.crc).map_err(Error::Serialize)?;

        // Arm correlation before the frame hits the wire, so the reply cannot
        // be observed before the runner knows to expect it.
        self.pending.set(Some((expect, seq)));
        if let Err(e) = self.tx.write_all(&self.buf[..len]).await {
            self.pending.set(None);
            return Err(Error::Write(e.kind()));
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
    pub async fn write<T: Serialize + ?Sized>(
        &mut self,
        addr: Address,
        val: &T,
    ) -> Result<(), Error> {
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
    pub async fn read<T: for<'a> Deserialize<'a>>(
        &mut self,
        addr: Address,
        wlen: u8,
    ) -> Result<T, Error> {
        self.transfer(
            Expect::Read { addr, wlen },
            |buf, crc| {
                Ok(Request::with_slice(buf, addr, Read(wlen))?
                    .finalize(crc)?
                    .len())
            },
            |bytes| {
                Content::from_bytes(bytes)
                    .take::<T>()
                    .map_err(|e| Error::Deserialize(addr, e))
            },
        )
        .await
    }
}

impl<'d, const MTU: usize> Events<'d, MTU> {
    /// Waits for the next unsolicited data frame.
    ///
    /// The returned [`Event`] borrows this handle and releases its queue slot
    /// on drop. Cancel-safe: dropping the future before it completes consumes
    /// nothing.
    pub async fn receive(&mut self) -> Event<'_, 'd, MTU> {
        let (addr, wlen) = {
            let slot = self.event_receiver.receive().await;
            (slot.addr, slot.wlen)
        };
        Event {
            ch: &mut self.event_receiver,
            addr,
            wlen,
            pos: 0,
        }
    }

    /// Returns the next queued unsolicited frame, or `None` if the queue is
    /// empty.
    pub fn try_receive(&mut self) -> Option<Event<'_, 'd, MTU>> {
        let (addr, wlen) = {
            let slot = self.event_receiver.try_receive()?;
            (slot.addr, slot.wlen)
        };
        Some(Event {
            ch: &mut self.event_receiver,
            addr,
            wlen,
            pos: 0,
        })
    }
}

/// Zero-copy view of one unsolicited data frame.
///
/// Deserialize the payload with [`Event::take`] (repeatedly for concatenated
/// values). The underlying queue slot is released when this guard is dropped.
pub struct Event<'a, 'd, const MTU: usize = DEFAULT_MTU> {
    ch: &'a mut Receiver<'d, NoopRawMutex, EventFrame<MTU>>,
    addr: Address,
    wlen: u8,
    pos: usize,
}

impl<'a, 'd, const MTU: usize> Event<'a, 'd, MTU> {
    /// The frame's variable address.
    pub fn addr(&self) -> Address {
        self.addr
    }

    /// The frame's payload length in words.
    pub fn wlen(&self) -> u8 {
        self.wlen
    }

    /// Deserializes the next `T` from the payload, advancing past it.
    pub fn take<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T, Error> {
        let addr = self.addr;
        let pos = self.pos;
        // Field access rather than `self.slot()` so `self.pos` stays free to
        // update below.
        let slot = self
            .ch
            .try_receive()
            .expect("event slot present until drop");
        let mut content = Content::from_bytes(&slot.buf[pos..slot.len]);
        let before = content.len();
        let out = content
            .take::<T>()
            .map_err(|e| Error::Deserialize(addr, e))?;
        self.pos += before - content.len();
        Ok(out)
    }

    /// The not-yet-taken remainder of the payload.
    pub fn content(&mut self) -> Content<'_> {
        let pos = self.pos;
        let slot = self.slot();
        Content::from_bytes(&slot.buf[pos..slot.len])
    }

    /// Re-peeks the queue slot this guard was constructed from. The slot stays
    /// at the front of the queue until `receive_done` in [`Drop`], and this
    /// guard exclusively borrows the receiver, so it is always present.
    fn slot(&mut self) -> &mut EventFrame<MTU> {
        self.ch
            .try_receive()
            .expect("event slot present until drop")
    }
}

impl<'a, 'd, const MTU: usize> Drop for Event<'a, 'd, MTU> {
    fn drop(&mut self) {
        self.ch.receive_done();
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
    /// - [`Error::Accumulation`]: framing noise (stray bytes, a partial frame
    ///   at startup, a CRC mismatch). Restarting resynchronizes on the next
    ///   frame header. The partially accumulated frame and the unprocessed
    ///   remainder of the current read window are lost; a request whose
    ///   response was in that data hangs until the caller's timeout and can
    ///   then be retried.
    /// - [`Error::EventQueueFull`]: an unsolicited frame arrived while the
    ///   event queue was full. That frame is lost; restarting resumes normal
    ///   operation. Size the queue with the `E` const on [`Resources`].
    /// - [`Error::Read`] / [`Error::UnexpectedEof`]: the transport itself
    ///   failed; restart only after the underlying stream has been repaired.
    ///
    /// Transmission is unaffected by any of this — [`Control`] owns the
    /// transmitter — so a restart never truncates an outgoing frame. Request
    /// correlation lives in shared state rather than in this future, so it
    /// survives a restart untouched.
    pub async fn run(&mut self, rx_buf: &mut [u8]) -> Result<Infallible, Error> {
        let mut acc = Accumulator::<MTU>::new(self.crc);
        loop {
            let n = self
                .rx
                .read(rx_buf)
                .await
                .map_err(|e| Error::Read(e.kind()))?;
            if n == 0 {
                return Err(Error::UnexpectedEof);
            }

            let mut window: &[u8] = &rx_buf[..n];
            while !window.is_empty() {
                window = match acc.feed(window) {
                    FeedResult::Consumed => break,
                    FeedResult::Error(e, _) => return Err(Error::Accumulation(e)),
                    FeedResult::Success(response, rest) => {
                        match response {
                            Response::Ack => {
                                if let Some((Expect::Ack, seq)) = self.pending.get() {
                                    self.pending.set(None);
                                    let slot = self.resp_sender.send().await;
                                    slot.seq = seq;
                                    slot.len = 0;
                                    self.resp_sender.send_done();
                                }
                                // Unmatched ACKs belong to cancelled requests;
                                // drop them.
                            }
                            Response::Data {
                                addr,
                                wlen,
                                content,
                            } => {
                                if let Some((Expect::Read { addr: a, wlen: w }, seq)) =
                                    self.pending.get()
                                    && a == addr
                                    && w == wlen
                                {
                                    self.pending.set(None);
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
                                    let bytes = content.as_bytes();
                                    let Some(slot) = self.event_sender.try_send() else {
                                        return Err(Error::EventQueueFull);
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
