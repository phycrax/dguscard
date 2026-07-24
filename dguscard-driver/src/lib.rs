#![no_std]

//! Async transport driver for DWIN DGUS T5L displays.
//!
//! Drives a DGUS display over any [`embedded_io_async`] byte stream, using
//! [`dguscard`] as the sans-io frame codec. The split follows `embassy-net`:
//! [`new`] takes caller-provided [`Stack`] storage and returns a [`Device`]
//! handle plus a [`Runner`] whose [`Runner::run`] future owns **both** UART
//! halves and must be polled continuously — typically from its own task.
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
//! - DGUS has no wire-level sequence numbers. If the display itself answers
//!   *late* — after a request was cancelled and a new one armed — a stale
//!   response can still satisfy a newer request of the same shape. Choose
//!   timeouts generously enough that the display has answered or never will.
//! - A read response is matched by `(addr, wlen)`. A spontaneous frame with the
//!   same address *and* length as an in-flight read is indistinguishable from
//!   its response; avoid reading from variables the display also auto-uploads.
//! - If the event queue is full because the application is not draining it,
//!   [`Runner::run`] returns [`Error::EventQueueFull`] rather than blocking
//!   request completion or silently dropping the frame. Size the queue with the
//!   `E` const on [`Stack`].
//!
//! [`NoopRawMutex`]: embassy_sync::blocking_mutex::raw::NoopRawMutex

use core::cell::Cell;
use core::convert::Infallible;
use core::mem::MaybeUninit;

pub use dguscard::Address;
pub use dguscard::response::Content;

use dguscard::request::{Read, Request, Write};
use dguscard::response::{Accumulator, FeedResult, Response};
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::zerocopy_channel::{Channel, Receiver, Sender};
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
    /// A read response did not fit in the MTU-sized response slot.
    ResponseTooLarge,
    /// An unsolicited frame arrived while the event queue was full.
    EventQueueFull,
}

/// Stack configuration.
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

/// What the in-flight operation is waiting for, moved from the handle to the
/// runner alongside the outgoing frame.
#[derive(Debug, Clone, Copy)]
enum Expect {
    /// A write's "OK" ACK.
    Ack,
    /// A read's data response, correlated by address and word length.
    Read { addr: Address, wlen: u8 },
}

/// A queued outgoing frame (handle → runner).
struct TxFrame<const MTU: usize> {
    expect: Expect,
    seq: u8,
    len: usize,
    buf: [u8; MTU],
}

impl<const MTU: usize> TxFrame<MTU> {
    const INIT: Self = Self {
        expect: Expect::Ack,
        seq: 0,
        len: 0,
        buf: [0; MTU],
    };
}

/// A completed operation's result (runner → handle). `len == 0` is a bare ACK.
struct RxFrame<const MTU: usize> {
    seq: u8,
    len: usize,
    buf: [u8; MTU],
}

impl<const MTU: usize> RxFrame<MTU> {
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

/// The three zero-copy channels, borrowing the buffer arrays in [`Stack`].
struct StateInner<'a, const MTU: usize> {
    tx: Channel<'a, NoopRawMutex, TxFrame<MTU>>,
    resp: Channel<'a, NoopRawMutex, RxFrame<MTU>>,
    events: Channel<'a, NoopRawMutex, EventFrame<MTU>>,
}

/// Caller-provided storage for the driver, like `embassy_net::StackResources`.
///
/// `MTU` bounds the largest sendable/receivable frame (dguscard accepts
/// 5..=256). `N` is the depth of the request/response channels; the default of
/// 1 is enough for the single-in-flight handle. `E` is the depth of the
/// unsolicited event queue — size it for the worst-case burst of touch events
/// between two [`Events::receive`] calls.
pub struct Stack<const MTU: usize = DEFAULT_MTU, const N: usize = 1, const E: usize = 4> {
    tx_bufs: [TxFrame<MTU>; N],
    resp_bufs: [RxFrame<MTU>; N],
    event_bufs: [EventFrame<MTU>; E],
    inner: MaybeUninit<StateInner<'static, MTU>>,
    crc: bool,
}

impl<const MTU: usize, const N: usize, const E: usize> Stack<MTU, N, E> {
    /// Creates new driver storage.
    pub const fn new(config: Config) -> Self {
        Self {
            tx_bufs: [TxFrame::INIT; N],
            resp_bufs: [RxFrame::INIT; N],
            event_bufs: [EventFrame::INIT; E],
            inner: MaybeUninit::uninit(),
            crc: config.crc,
        }
    }
}

impl<const MTU: usize, const N: usize, const E: usize> Default for Stack<MTU, N, E> {
    fn default() -> Self {
        Self::new(Config::new())
    }
}

/// Application-facing handle bundling [`Control`] and [`Events`].
///
/// Convenient for simple sequential use; note that [`Device::event`] borrows
/// the whole handle, so a held [`Event`] blocks [`Device::write`] /
/// [`Device::read`]. Call [`Device::split`] when one task needs to issue
/// requests while an event is being processed.
pub struct Device<'d, const MTU: usize = DEFAULT_MTU> {
    control: Control<'d, MTU>,
    events: Events<'d, MTU>,
}

/// Request half of the handle: serializes requests and awaits their correlated
/// completion. Does not own the transport (the [`Runner`] does).
pub struct Control<'d, const MTU: usize = DEFAULT_MTU> {
    tx_sender: Sender<'d, NoopRawMutex, TxFrame<MTU>>,
    resp_receiver: Receiver<'d, NoopRawMutex, RxFrame<MTU>>,
    seq: u8,
    crc: bool,
}

/// Event half of the handle: receives unsolicited data frames queued by the
/// [`Runner`].
pub struct Events<'d, const MTU: usize = DEFAULT_MTU> {
    event_receiver: Receiver<'d, NoopRawMutex, EventFrame<MTU>>,
}

/// RX+TX worker. [`Runner::run`] must be polled continuously for requests and
/// event delivery to make progress.
pub struct Runner<'d, RX: IoRead, TX: IoWrite, const MTU: usize = DEFAULT_MTU> {
    rx: RX,
    tx: TX,
    tx_receiver: Receiver<'d, NoopRawMutex, TxFrame<MTU>>,
    resp_sender: Sender<'d, NoopRawMutex, RxFrame<MTU>>,
    event_sender: Sender<'d, NoopRawMutex, EventFrame<MTU>>,
    crc: bool,
}

/// Creates a driver instance from a split transport.
///
/// Returns the [`Device`] handle and the [`Runner`] that drives the UART.
pub fn new<'d, RX, TX, const MTU: usize, const N: usize, const E: usize>(
    stack: &'d mut Stack<MTU, N, E>,
    rx: RX,
    tx: TX,
) -> (Device<'d, MTU>, Runner<'d, RX, TX, MTU>)
where
    RX: IoRead,
    TX: IoWrite,
{
    let crc = stack.crc;

    // Self-referential init, mirroring `embassy-net-driver-channel`: the
    // channels in `StateInner` borrow `stack.tx_bufs` / `stack.resp_bufs` /
    // `stack.event_bufs`, which are fields of the same `stack`.
    //
    // SAFETY: the borrow laundered from `'static` to `'d` is sound because the
    // returned handles carry `'d`, so `stack` cannot move or be reused while any
    // of the internal references are live, and those references are never
    // touched once the `'d` borrow ends.
    let inner_uninit: *mut MaybeUninit<StateInner<'d, MTU>> =
        (&mut stack.inner as *mut MaybeUninit<StateInner<'static, MTU>>).cast();
    let inner = unsafe { &mut *inner_uninit }.write(StateInner {
        tx: Channel::new(&mut stack.tx_bufs[..]),
        resp: Channel::new(&mut stack.resp_bufs[..]),
        events: Channel::new(&mut stack.event_bufs[..]),
    });

    let (tx_sender, tx_receiver) = inner.tx.split();
    let (resp_sender, resp_receiver) = inner.resp.split();
    let (event_sender, event_receiver) = inner.events.split();

    let device = Device {
        control: Control {
            tx_sender,
            resp_receiver,
            seq: 0,
            crc,
        },
        events: Events { event_receiver },
    };
    let runner = Runner {
        rx,
        tx,
        tx_receiver,
        resp_sender,
        event_sender,
        crc,
    };
    (device, runner)
}

impl<'d, const MTU: usize> Device<'d, MTU> {
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
    pub fn split(self) -> (Control<'d, MTU>, Events<'d, MTU>) {
        (self.control, self.events)
    }
}

impl<'d, const MTU: usize> Control<'d, MTU> {
    /// Discards completions left over from cancelled (timed-out) requests, then
    /// stamps and returns the sequence number for a new request.
    fn begin_request(&mut self) -> u8 {
        while self.resp_receiver.try_receive().is_some() {
            self.resp_receiver.receive_done();
        }
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    /// Sends one request frame built by `build` and awaits its correlated
    /// completion, mapping the completion payload with `complete`.
    async fn transfer<R>(
        &mut self,
        expect: Expect,
        build: impl FnOnce(&mut [u8], bool) -> Result<usize, dguscard::Error>,
        complete: impl FnOnce(&[u8]) -> Result<R, Error>,
    ) -> Result<R, Error> {
        let seq = self.begin_request();
        let frame = self.tx_sender.send().await;
        frame.len = build(&mut frame.buf, self.crc).map_err(Error::Serialize)?;
        frame.expect = expect;
        frame.seq = seq;
        self.tx_sender.send_done();

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
    /// Cancel-safe: a completion arriving after this future is dropped is
    /// discarded by the next request.
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
    /// Cancel-safe: a completion arriving after this future is dropped is
    /// discarded by the next request.
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

impl<'d, RX: IoRead, TX: IoWrite, const MTU: usize> Runner<'d, RX, TX, MTU> {
    /// Concurrently drives the RX and TX loops.
    ///
    /// Data frames that complete an in-flight request are handed back to
    /// [`Control`]; all other data frames are queued for [`Events`]. Returns
    /// only on the first transport or framing error, or with
    /// [`Error::EventQueueFull`] if an unsolicited frame arrives while the
    /// event queue is full.
    pub async fn run(&mut self) -> Result<Infallible, Error> {
        let rx = &mut self.rx;
        let tx = &mut self.tx;
        let rx_buf = &mut [0; MTU];
        let tx_receiver = &mut self.tx_receiver;
        let resp_sender = &mut self.resp_sender;
        let events_sender = &mut self.event_sender;
        let crc = self.crc;

        // Correlation between the two futures
        let pending = Cell::new(None);

        let tx_fut = async {
            loop {
                let frame = tx_receiver.receive().await;
                // Arm correlation before the frame hits the wire, so the ACK
                // cannot be observed before we know to expect it.
                pending.set(Some((frame.expect, frame.seq)));
                let r = tx.write_all(&frame.buf[..frame.len]).await;
                tx_receiver.receive_done();
                if let Err(e) = r {
                    return Err(Error::Write(e.kind()));
                }
            }
        };

        let rx_fut = async {
            let mut acc = Accumulator::<MTU>::new(crc);
            loop {
                let n = rx
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
                                    if let Some((Expect::Ack, seq)) = pending.get() {
                                        pending.set(None);
                                        let slot = resp_sender.send().await;
                                        slot.seq = seq;
                                        slot.len = 0;
                                        resp_sender.send_done();
                                    }
                                    // Unmatched ACKs belong to cancelled
                                    // requests; drop them.
                                }
                                Response::Data {
                                    addr,
                                    wlen,
                                    content,
                                } => {
                                    match pending.get() {
                                        Some((Expect::Read { addr: a, wlen: w }, seq))
                                            if a == addr && w == wlen =>
                                        {
                                            pending.set(None);
                                            let bytes = content.as_bytes();
                                            if bytes.len() > MTU {
                                                return Err(Error::ResponseTooLarge);
                                            }
                                            let slot = resp_sender.send().await;
                                            slot.buf[..bytes.len()].copy_from_slice(bytes);
                                            slot.len = bytes.len();
                                            slot.seq = seq;
                                            resp_sender.send_done();
                                        }
                                        _ => {
                                            // Unsolicited: queue for `Events`.
                                            let bytes = content.as_bytes();
                                            if bytes.len() > MTU {
                                                return Err(Error::ResponseTooLarge);
                                            }
                                            let Some(slot) = events_sender.try_send() else {
                                                return Err(Error::EventQueueFull);
                                            };
                                            slot.addr = addr;
                                            slot.wlen = wlen;
                                            slot.buf[..bytes.len()].copy_from_slice(bytes);
                                            slot.len = bytes.len();
                                            events_sender.send_done();
                                        }
                                    }
                                }
                            }
                            rest
                        }
                    };
                }
            }
        };

        match select(rx_fut, tx_fut).await {
            Either::First(r) => r,
            Either::Second(r) => r,
        }
    }
}
