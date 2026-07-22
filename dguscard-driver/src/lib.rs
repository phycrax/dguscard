#![no_std]

use core::convert::Infallible;

use dguscard::response::{Accumulator, FeedResult};
use embedded_io_async::Error as _;

const DEFAULT_MTU: usize = 128;

/// Driver errors.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Frame accumulation failed.
    Accumulation(dguscard::Error),
    /// The transport reached EOF.
    UnexpectedEof,
    /// Transport read error.
    Read(embedded_io_async::ErrorKind),
    /// Transport write error.
    Write(embedded_io_async::ErrorKind),
}

/// Create a new driver.
pub fn new<
    'r,
    R: embedded_io_async::Read,
    W: embedded_io_async::Write,
    const RX_MTU: usize,
    const TX_MTU: usize,
    const ACC_MTU: usize,
>(
    res: &'r mut Resources<RX_MTU, TX_MTU, ACC_MTU>,
    rx: R,
    _tx: W,
) -> Runner<'r, R, ACC_MTU> {
    Runner {
        uart_rx: rx,
        rx_buf: &mut res.rx_buf[..],
        accumulator: &mut res.accumulator,
    }
}

pub struct Resources<
    const RX_MTU: usize = DEFAULT_MTU,
    const TX_MTU: usize = DEFAULT_MTU,
    const ACC_MTU: usize = DEFAULT_MTU,
> {
    pub(crate) rx_buf: [u8; RX_MTU],
    pub(crate) _tx_buf: [u8; TX_MTU],
    pub(crate) accumulator: Accumulator<ACC_MTU>,
}

impl<const RX_MTU: usize, const TX_MTU: usize, const ACC_MTU: usize>
    Resources<RX_MTU, TX_MTU, ACC_MTU>
{
    /// Allocate driver resources
    pub const fn new(crc: bool) -> Self {
        Self {
            rx_buf: [0; RX_MTU],
            _tx_buf: [0; TX_MTU],
            accumulator: Accumulator::new(crc),
        }
    }
}

impl<const RX_MTU: usize, const TX_MTU: usize, const ACC_MTU: usize> Default
    for Resources<RX_MTU, TX_MTU, ACC_MTU>
{
    /// CRC enabled by default
    fn default() -> Self {
        Self::new(true)
    }
}

pub struct Runner<'r, R: embedded_io_async::Read, const MTU: usize = DEFAULT_MTU> {
    uart_rx: R,
    rx_buf: &'r mut [u8],
    accumulator: &'r mut Accumulator<MTU>,
}

impl<'r, R: embedded_io_async::Read, const MTU: usize> Runner<'r, R, MTU> {
    pub async fn run(&mut self) -> Result<Infallible, Error> {
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
                    FeedResult::Success(_response, remaining) => {
                        //TODO match response {}

                        remaining
                    }
                };
            }
        }
    }
}
