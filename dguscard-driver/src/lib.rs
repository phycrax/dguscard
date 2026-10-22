#![no_std]

mod route;

use core::cell::Cell;
use core::convert::Infallible;
use core::mem::MaybeUninit;

use dguscard::request::{Read, Request, Write};
pub use dguscard::response::Content;
use dguscard::response::{FeedResult, Response, SliceAccumulator};
pub use dguscard::{Address, Crc, request, response};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::zerocopy_channel::{Channel, Receiver, Sender};
use embedded_io_async::{Error as _, Read as IoRead, Write as IoWrite};
pub use route::{RouteError, Router};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// [`Runner::run`] was not executing when the call was made, so nothing would have received the reply.
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
    /// An ack arrived that matched no request in flight.
    UnmatchedAck,
    /// The reply to a cancelled read arrived and was dropped.
    LateReply(Address, u8),
    /// Transport read reached EOF.
    UnexpectedEof,
    /// Transport read error.
    Io(embedded_io_async::ErrorKind),
    /// Routing failed for an unsolicited frame.
    Route(RouteError),
}

/// Driver configuration, passed to [`new`].
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// Append/verify CRC on each frame.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    Ack,
    Read { addr: Address, wlen: u8 },
}

#[derive(Clone, Copy)]
struct State {
    pending: Option<(Expect, u8)>,
    abandoned_read: Option<(Address, u8)>,
    running: bool,
}

struct Shared<M: RawMutex> {
    state: Mutex<M, Cell<State>>,
}

impl<M: RawMutex> Shared<M> {
    const fn new() -> Self {
        Self {
            state: Mutex::new(Cell::new(State {
                pending: None,
                abandoned_read: None,
                running: false,
            })),
        }
    }

    /// Runs `f` on the state under the lock.
    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        self.state.lock(|cell| {
            let mut state = cell.get();
            let out = f(&mut state);
            cell.set(state);
            out
        })
    }
}

struct RunGuard<'d, M: RawMutex> {
    shared: &'d Shared<M>,
}

impl<'d, M: RawMutex> RunGuard<'d, M> {
    fn new(shared: &'d Shared<M>) -> Self {
        shared.with(|s| s.running = true);
        Self { shared }
    }
}

impl<M: RawMutex> Drop for RunGuard<'_, M> {
    fn drop(&mut self) {
        self.shared.with(|s| s.running = false);
    }
}

struct PendingGuard<'d, M: RawMutex> {
    shared: &'d Shared<M>,
}

impl<'d, M: RawMutex> PendingGuard<'d, M> {
    fn new(shared: &'d Shared<M>, expect: Expect, seq: u8) -> Self {
        shared.with(|s| s.pending = Some((expect, seq)));
        Self { shared }
    }
}

impl<M: RawMutex> Drop for PendingGuard<'_, M> {
    fn drop(&mut self) {
        self.shared.with(|s| {
            if let Some((Expect::Read { addr, wlen }, _)) = s.pending.take() {
                s.abandoned_read = Some((addr, wlen));
            }
        });
    }
}

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

struct StateInner<'a, M: RawMutex, const MTU: usize> {
    resp: Channel<'a, M, ResponseFrame<MTU>>,
    shared: Shared<M>,
}

pub struct Resources<M: RawMutex, const MTU: usize> {
    resp_bufs: [ResponseFrame<MTU>; 1],
    req_buf: [u8; MTU],
    acc_buf: [u8; MTU],
    inner: MaybeUninit<StateInner<'static, M, MTU>>,
}

impl<M: RawMutex, const MTU: usize> Resources<M, MTU> {
    /// Creates new driver resources.
    pub const fn new() -> Self {
        Self {
            resp_bufs: [ResponseFrame::INIT; 1],
            req_buf: [0; MTU],
            acc_buf: [0; MTU],
            inner: MaybeUninit::uninit(),
        }
    }
}

impl<M: RawMutex, const MTU: usize> Default for Resources<M, MTU> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Display<'d, M: RawMutex, TX: IoWrite, const MTU: usize> {
    tx: TX,
    buf: &'d mut [u8],
    resp_receiver: Receiver<'d, M, ResponseFrame<MTU>>,
    shared: &'d Shared<M>,
    seq: u8,
    crc: Option<Crc>,
}

pub struct Runner<'d, M: RawMutex, RX: IoRead, const MTU: usize> {
    rx: RX,
    acc: SliceAccumulator<'d>,
    resp_sender: Sender<'d, M, ResponseFrame<MTU>>,
    shared: &'d Shared<M>,
}

pub fn new<'d, M, RX, TX, const MTU: usize>(
    resources: &'d mut Resources<M, MTU>,
    rx: RX,
    tx: TX,
    config: Config,
) -> (Display<'d, M, TX, MTU>, Runner<'d, M, RX, MTU>)
where
    M: RawMutex,
    RX: IoRead,
    TX: IoWrite,
{
    let crc = if config.crc { Crc::on() } else { Crc::off() };

    let inner_uninit: *mut MaybeUninit<StateInner<'d, M, MTU>> =
        (&mut resources.inner as *mut MaybeUninit<StateInner<'static, M, MTU>>).cast();
    let inner = unsafe { &mut *inner_uninit }.write(StateInner {
        resp: Channel::new(&mut resources.resp_bufs[..]),
        shared: Shared::new(),
    });

    let (resp_sender, resp_receiver) = inner.resp.split();
    let shared = &inner.shared;

    let display = Display {
        tx,
        buf: &mut resources.req_buf[..],
        resp_receiver,
        shared,
        seq: 0,
        crc,
    };
    let runner = Runner {
        rx,
        acc: SliceAccumulator::new(&mut resources.acc_buf[..], crc),
        resp_sender,
        shared,
    };
    (display, runner)
}

impl<'d, M: RawMutex, TX: IoWrite, const MTU: usize> Display<'d, M, TX, MTU> {
    fn begin_request(&mut self) -> u8 {
        while let Some(slot) = self.resp_receiver.try_receive() {
            // Hey! You dropped here: unmatched response
            slot.receive_done();
        }
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    async fn transfer<R>(
        &mut self,
        expect: Expect,
        build: impl FnOnce(&mut [u8], Option<Crc>) -> Result<usize, request::Error>,
        complete: impl FnOnce(&[u8]) -> Result<R, Error>,
    ) -> Result<R, Error> {
        if !self.shared.with(|s| s.running) {
            return Err(Error::NotRunning);
        }

        let seq = self.begin_request();
        let len = build(&mut self.buf[..], self.crc).map_err(Error::Serialize)?;

        let _pending = PendingGuard::new(self.shared, expect, seq);
        if let Err(e) = self.tx.write_all(&self.buf[..len]).await {
            return Err(Error::Io(e.kind()));
        }

        loop {
            let resp = self.resp_receiver.receive().await;
            if resp.seq == seq {
                let out = complete(&resp.buf[..resp.len]);
                resp.receive_done();
                return out;
            }
            resp.receive_done();
        }
    }

    pub async fn write<T: Serialize + ?Sized>(&mut self, addr: Address, val: &T) -> Result<(), Error> {
        self.transfer(
            Expect::Ack,
            |buf, crc| {
                let mut req = Request::with_slice(buf, addr, Write, crc)?;
                req.push(val)?;
                Ok(req.finalize()?.len())
            },
            |_| Ok(()),
        )
        .await
    }

    pub async fn read<T: for<'a> Deserialize<'a>>(&mut self, addr: Address, wlen: u8) -> Result<T, Error> {
        self.transfer(
            Expect::Read { addr, wlen },
            |buf, crc| Ok(Request::with_slice(buf, addr, Read(wlen), crc)?.finalize()?.len()),
            |bytes| {
                Content::new(addr, wlen, bytes)
                    .take::<T>()
                    .map_err(|e| Error::Deserialize(addr, e))
            },
        )
        .await
    }
}

impl<'d, M: RawMutex, RX: IoRead, const MTU: usize> Runner<'d, M, RX, MTU> {
    pub async fn run<R: Router>(&mut self, buf: &mut [u8], router: &mut R) -> Result<Infallible, RunError> {
        let _guard = RunGuard::new(self.shared);
        loop {
            let n = self.rx.read(buf).await.map_err(|e| RunError::Io(e.kind()))?;
            if n == 0 {
                return Err(RunError::UnexpectedEof);
            }

            let mut failed: Option<RunError> = None;
            let mut window: &[u8] = &buf[..n];
            while !window.is_empty() {
                window = match self.acc.feed(window) {
                    FeedResult::Consumed => break,
                    FeedResult::Error(e, rest) => {
                        failed.get_or_insert(RunError::Accumulation(e));
                        rest
                    }
                    FeedResult::Success(response, rest) => {
                        match response {
                            Response::Ack => {
                                // Is it a matching ACK?
                                if let Some((_, seq)) =
                                    self.shared.with(|s| s.pending.take_if(|(e, _)| *e == Expect::Ack))
                                {
                                    let mut slot = self.resp_sender.send().await;
                                    slot.seq = seq;
                                    slot.len = 0;
                                    slot.send_done();
                                } else {
                                    failed.get_or_insert(RunError::UnmatchedAck);
                                }
                            }
                            Response::Data(content) => {
                                let (addr, wlen) = (content.addr(), content.wlen());
                                // Are we waiting for a response (with matching address, wlen, seq)?
                                if let Some((_, seq)) = self
                                    .shared
                                    .with(|s| s.pending.take_if(|(e, _)| *e == Expect::Read { addr, wlen }))
                                {
                                    let bytes = content.as_bytes();
                                    let mut slot = self.resp_sender.send().await;
                                    slot.buf[..bytes.len()].copy_from_slice(bytes);
                                    slot.len = bytes.len();
                                    slot.seq = seq;
                                    slot.send_done();
                                }
                                // Or the late reply to a read nobody waits for any more? Drop it.
                                else if self
                                    .shared
                                    .with(|s| s.abandoned_read.take_if(|r| *r == (addr, wlen)))
                                    .is_some()
                                {
                                    failed.get_or_insert(RunError::LateReply(addr, wlen));
                                }
                                // Or is this an unsolicited event?
                                else if let Err(e) = router.route(content) {
                                    // Routing failed. Only this frame is dropped, the rest of the window is still processed.
                                    failed.get_or_insert(RunError::Route(e));
                                }
                            }
                        }
                        rest
                    }
                };
            }

            if let Some(e) = failed {
                return Err(e);
            }
        }
    }
}
