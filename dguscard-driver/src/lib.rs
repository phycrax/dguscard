#![no_std]

mod dispatch;

use core::cell::Cell;
use core::convert::Infallible;
use core::mem::MaybeUninit;

use dguscard::request::{Read, Request, Write};
pub use dguscard::response::Content;
use dguscard::response::{FeedResult, Response, SliceAccumulator};
pub use dguscard::{Address, Crc, request, response};
pub use dispatch::{DispatchError, Dispatcher};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::zerocopy_channel::{Channel, Receiver, Sender};
use embedded_io_async::{Error as _, Read as IoRead, Write as IoWrite};
use serde::{Deserialize, Serialize};

const DEFAULT_MTU: usize = 128;

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
    /// [`Dispatch`] failed for an unsolicited frame.
    Dispatch(DispatchError),
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

struct Shared {
    pending: Cell<Option<(Expect, u8)>>,
    abandoned_read: Cell<Option<(Address, u8)>>,
    running: Cell<bool>,
}

impl Shared {
    const fn new() -> Self {
        Self {
            pending: Cell::new(None),
            abandoned_read: Cell::new(None),
            running: Cell::new(false),
        }
    }
}

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

struct PendingGuard<'d> {
    shared: &'d Shared,
}

impl<'d> PendingGuard<'d> {
    fn new(shared: &'d Shared, expect: Expect, seq: u8) -> Self {
        shared.pending.set(Some((expect, seq)));
        Self { shared }
    }
}

impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        if let Some((Expect::Read { addr, wlen }, _)) = self.shared.pending.take() {
            self.shared.abandoned_read.set(Some((addr, wlen)));
        }
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

struct StateInner<'a, const MTU: usize> {
    resp: Channel<'a, NoopRawMutex, ResponseFrame<MTU>>,
    shared: Shared,
}

pub struct Resources<const MTU: usize = DEFAULT_MTU> {
    resp_bufs: [ResponseFrame<MTU>; 1],
    req_buf: [u8; MTU],
    acc_buf: [u8; MTU],
    inner: MaybeUninit<StateInner<'static, MTU>>,
}

impl<const MTU: usize> Resources<MTU> {
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

impl<const MTU: usize> Default for Resources<MTU> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Display<'d, TX: IoWrite, const MTU: usize = DEFAULT_MTU> {
    tx: TX,
    buf: &'d mut [u8],
    resp_receiver: Receiver<'d, NoopRawMutex, ResponseFrame<MTU>>,
    shared: &'d Shared,
    seq: u8,
    crc: Option<Crc>,
}

pub struct Runner<'d, RX: IoRead, const MTU: usize = DEFAULT_MTU> {
    rx: RX,
    acc: SliceAccumulator<'d>,
    resp_sender: Sender<'d, NoopRawMutex, ResponseFrame<MTU>>,
    shared: &'d Shared,
}

pub fn new<'d, RX, TX, const MTU: usize>(
    resources: &'d mut Resources<MTU>,
    rx: RX,
    tx: TX,
    config: Config,
) -> (Display<'d, TX, MTU>, Runner<'d, RX, MTU>)
where
    RX: IoRead,
    TX: IoWrite,
{
    let crc = if config.crc { Crc::on() } else { Crc::off() };

    let inner_uninit: *mut MaybeUninit<StateInner<'d, MTU>> =
        (&mut resources.inner as *mut MaybeUninit<StateInner<'static, MTU>>).cast();
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

impl<'d, TX: IoWrite, const MTU: usize> Display<'d, TX, MTU> {
    fn begin_request(&mut self) -> u8 {
        while self.resp_receiver.try_receive().is_some() {
            // Hey! You dropped here: unmatched frame
            self.resp_receiver.receive_done();
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
        if !self.shared.running.get() {
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
                self.resp_receiver.receive_done();
                return out;
            }
            self.resp_receiver.receive_done();
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

impl<'d, RX: IoRead, const MTU: usize> Runner<'d, RX, MTU> {
    pub async fn run<D: Dispatcher>(
        &mut self,
        rx_buf: &mut [u8],
        ctx: &mut D::Context,
        dispatcher: &mut D,
    ) -> Result<Infallible, RunError> {
        let _guard = RunGuard::new(self.shared);
        loop {
            let n = self.rx.read(rx_buf).await.map_err(|e| RunError::Io(e.kind()))?;
            if n == 0 {
                return Err(RunError::UnexpectedEof);
            }

            let mut failed: Option<RunError> = None;
            let mut window: &[u8] = &rx_buf[..n];
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
                                if let Some((Expect::Ack, seq)) = self.shared.pending.get() {
                                    self.shared.pending.set(None);
                                    let slot = self.resp_sender.send().await;
                                    slot.seq = seq;
                                    slot.len = 0;
                                    self.resp_sender.send_done();
                                } else {
                                    failed.get_or_insert(RunError::UnmatchedAck);
                                }
                            }
                            Response::Data(content) => {
                                // Are we waiting for a response (with matching address, wlen, seq)?
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
                                }
                                // Or the late reply to a read nobody waits for any more? Drop it.
                                else if self.shared.abandoned_read.get() == Some((content.addr(), content.wlen())) {
                                    self.shared.abandoned_read.set(None);
                                    failed.get_or_insert(RunError::LateReply(content.addr(), content.wlen()));
                                }
                                // Or is this an unsolicited event?
                                else if let Err(e) = dispatcher.dispatch(ctx, content) {
                                    // Dispatch failed. Only this frame is dropped, the rest of the window is still processed.
                                    failed.get_or_insert(RunError::Dispatch(e));
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
