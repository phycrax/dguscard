#![no_std]

mod route;

use core::cell::Cell;
use core::convert::Infallible;

use dguscard::request::{Read, Request, Write};
pub use dguscard::response::Content;
use dguscard::response::{FeedResult, Response, SliceAccumulator};
pub use dguscard::{Address, Crc, request, response};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use embassy_sync::signal::Signal;
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
    /// `(seq, len)` of the reply the runner put in the reply buffer, `len` is 0 for an ack.
    replied: Signal<M, (u8, usize)>,
}

impl<M: RawMutex> Shared<M> {
    const fn new() -> Self {
        Self {
            state: Mutex::new(Cell::new(State {
                pending: None,
                abandoned_read: None,
                running: false,
            })),
            replied: Signal::new(),
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

pub struct Resources<M: RawMutex, const MTU: usize> {
    req_buf: [u8; MTU],
    acc_buf: [u8; MTU],
    reply_buf: AsyncMutex<M, [u8; MTU]>,
    shared: Shared<M>,
}

impl<M: RawMutex, const MTU: usize> Resources<M, MTU> {
    /// Creates new driver resources.
    pub const fn new() -> Self {
        Self {
            req_buf: [0; MTU],
            acc_buf: [0; MTU],
            reply_buf: AsyncMutex::new([0; MTU]),
            shared: Shared::new(),
        }
    }
}

impl<M: RawMutex, const MTU: usize> Default for Resources<M, MTU> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Display<'d, M: RawMutex, TX: IoWrite> {
    tx: TX,
    buf: &'d mut [u8],
    reply_buf: &'d AsyncMutex<M, [u8]>,
    shared: &'d Shared<M>,
    seq: u8,
    crc: Option<Crc>,
}

pub struct Runner<'d, M: RawMutex, RX: IoRead> {
    rx: RX,
    acc: SliceAccumulator<'d>,
    reply_buf: &'d AsyncMutex<M, [u8]>,
    shared: &'d Shared<M>,
}

pub fn new<'d, M, RX, TX, const MTU: usize>(
    resources: &'d mut Resources<M, MTU>,
    rx: RX,
    tx: TX,
    config: Config,
) -> (Display<'d, M, TX>, Runner<'d, M, RX>)
where
    M: RawMutex,
    RX: IoRead,
    TX: IoWrite,
{
    let crc = if config.crc { Crc::on() } else { Crc::off() };

    let Resources {
        req_buf,
        acc_buf,
        reply_buf,
        shared,
    } = resources;
    // Nothing from a previous driver over these resources carries over.
    *shared = Shared::new();
    let (reply_buf, shared) = (&*reply_buf, &*shared);

    let display = Display {
        tx,
        buf: &mut req_buf[..],
        reply_buf,
        shared,
        seq: 0,
        crc,
    };
    let runner = Runner {
        rx,
        acc: SliceAccumulator::new(&mut acc_buf[..], crc),
        reply_buf,
        shared,
    };
    (display, runner)
}

impl<'d, M: RawMutex, TX: IoWrite> Display<'d, M, TX> {
    fn begin_request(&mut self) -> u8 {
        // Hey! You dropped here: the reply to a cancelled request
        self.shared.replied.reset();
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
            let (replied, len) = self.shared.replied.wait().await;
            // Or the reply to a request cancelled once the runner had taken it? Ours is still to come.
            if replied == seq {
                let reply = self.reply_buf.lock().await;
                return complete(&reply[..len]);
            }
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

impl<'d, M: RawMutex, RX: IoRead> Runner<'d, M, RX> {
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
                                    self.shared.replied.signal((seq, 0));
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
                                    self.reply_buf.lock().await[..bytes.len()].copy_from_slice(bytes);
                                    self.shared.replied.signal((seq, bytes.len()));
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
