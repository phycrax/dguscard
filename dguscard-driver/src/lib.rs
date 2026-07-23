#![no_std]

mod router;

use core::convert::Infallible;

pub use dguscard::Address;
pub use dguscard::response::Content;
pub use router::Router;

use dguscard::response::{Accumulator, FeedResult, Response};
use embedded_io_async::Error as _;

const DEFAULT_MTU: usize = 128;

/// Driver errors.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Frame accumulation failed.
    Accumulation(dguscard::Error),
    /// Response payload could not be deserialized into the mapped type.
    Deserialize(Address, dguscard::Error),
    /// Transport read reached EOF.
    UnexpectedEof,
    /// Transport read error.
    Read(embedded_io_async::ErrorKind),
    /// Transport write error.
    Write(embedded_io_async::ErrorKind),
    /// A data frame's `(addr, wlen)` has no handler in the routing table.
    ///
    /// Only produced when the [`router!`] table sets `ignore_unhandled: false`.
    Unhandled(Address, u8),
}

/// Stack configuration.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// Verify a trailing CRC on each received frame.
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

/// Create a new driver.
pub fn new<
    'r,
    RX: embedded_io_async::Read,
    TX: embedded_io_async::Write,
    const RX_MTU: usize,
    const TX_MTU: usize,
    const ACC_MTU: usize,
>(
    res: &'r mut Stack<RX_MTU, TX_MTU, ACC_MTU>,
    rx: RX,
    _tx: TX,
) -> Runner<'r, RX, ACC_MTU> {
    Runner {
        uart_rx: rx,
        rx_buf: &mut res.rx_buf[..],
        accumulator: &mut res.accumulator,
    }
}

pub struct Stack<
    const RX_MTU: usize = DEFAULT_MTU,
    const TX_MTU: usize = DEFAULT_MTU,
    const ACC_MTU: usize = DEFAULT_MTU,
> {
    pub(crate) rx_buf: [u8; RX_MTU],
    pub(crate) _tx_buf: [u8; TX_MTU],
    pub(crate) accumulator: Accumulator<ACC_MTU>,
}

impl<const RX_MTU: usize, const TX_MTU: usize, const ACC_MTU: usize>
    Stack<RX_MTU, TX_MTU, ACC_MTU>
{
    /// Create new driver stack
    pub const fn new(config: Config) -> Self {
        Self {
            rx_buf: [0; RX_MTU],
            _tx_buf: [0; TX_MTU],
            accumulator: Accumulator::new(config.crc),
        }
    }
}

impl<const RX_MTU: usize, const TX_MTU: usize, const ACC_MTU: usize> Default
    for Stack<RX_MTU, TX_MTU, ACC_MTU>
{
    /// Default [`Config`]
    fn default() -> Self {
        Self::new(Config::new())
    }
}

pub struct Runner<'r, RX: embedded_io_async::Read, const MTU: usize = DEFAULT_MTU> {
    uart_rx: RX,
    rx_buf: &'r mut [u8],
    accumulator: &'r mut Accumulator<MTU>,
}

impl<'r, RX: embedded_io_async::Read, const MTU: usize> Runner<'r, RX, MTU> {
    /// Runs the receive loop, routing data frames through `router`.
    ///
    /// The context is passed here rather than at construction so it may borrow
    /// task-local state. Returns only on transport or framing errors.
    pub async fn run<R: Router>(
        &mut self,
        router: &mut R,
        cx: &mut R::Context,
    ) -> Result<Infallible, Error> {
        loop {
            let n = self
                .uart_rx
                .read(self.rx_buf)
                .await
                .map_err(|e| Error::Read(e.kind()))?;

            // TODO assess whether this check is necessary
            if n == 0 {
                return Err(Error::UnexpectedEof);
            }

            let mut window = &self.rx_buf[..n];
            while !window.is_empty() {
                window = match self.accumulator.feed(window) {
                    FeedResult::Consumed => break,
                    FeedResult::Error(e, _) => return Err(Error::Accumulation(e)),
                    FeedResult::Success(response, remaining) => {
                        match response {
                            Response::Ack => (), //TODO
                            Response::Data {
                                addr,
                                wlen,
                                mut content,
                            } => {
                                router.route(addr, wlen, &mut content, cx)?;
                            }
                        }

                        remaining
                    }
                };
            }
        }
    }
}
