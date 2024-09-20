//! An accumulator used to collect chunked DGUS RX frame.

use crate::error::{Error, Result};
use crate::rx::RxFrame;
use crate::{CRC, HEADER};

/// An accumulator used to collect chunked DGUS RX frame.
///
/// This is often useful when you receive "parts" of the frame at a time, for example when draining
/// a serial port buffer that may not contain an entire uninterrupted frame.
///
/// # Examples
///
/// Collect a frame by reading chunks then deserialize a struct from the frame.
///
/// ```rust
/// use dguscard::rx::{Accumulator, FeedResult};
/// use std::io::Read;
/// # #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
/// # struct MyData {
/// #     a: u16,
/// #     b: bool,
/// #     c: u32,
/// # }
/// let mut uart = /* Anything that implements the `Read` trait */
/// # &[0x5A, 0xA5, 12, 0x83, 0x12, 0x34, 4, 0xAA, 0xBB, 0x00, 0x01, 0xCC, 0xDD, 0xEE, 0xFF][..];
///
/// let mut raw_buf = [0u8; 32];
/// // Create a new Accumulator with CRC check disabled.
/// let mut dgus_buf: Accumulator<128> = Accumulator::new(false);
///
/// while let Ok(ct) = uart.read(&mut raw_buf) {
///     // Finished reading input
///     if ct == 0 {
///         break;
///     }
///
///     let buf = &raw_buf[..ct];
///     let mut window = &buf[..];
///
///     'dgus: while !window.is_empty() {
///         window = match dgus_buf.feed(&window) {
///             FeedResult::Consumed => break 'dgus,
///             FeedResult::Error(error, remaining) => {
///                 // Do something with the error here.
///
///                 dbg!(error);
///
///                 remaining
///             },
///             FeedResult::Success(mut frame, remaining) => {
///                 // Deserialize the content of `frame: RxFrame` here.
///                 
///                 let data: MyData = frame.take().unwrap();
///     
///                 dbg!(data);
///
///                 remaining
///             }
///         };
///     }
/// }
/// ```
///
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Accumulator<const N: usize> {
    buf: [u8; N],
    idx: usize,
    crc: bool,
    state: FeedState,
}

/// The result of feeding the accumulator.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FeedResult<'de, 'a> {
    /// Consumed all data, still pending.
    Consumed,
    /// Accumulation failed. Contains remaining section of input, if any.
    Error(Error, &'a [u8]),
    /// Accumulation successful. Contains a frame and remaining section of input, if any.
    Success(RxFrame<'de>, &'a [u8]),
}

/// The internal state of feeding the accumulator.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum FeedState {
    Empty,
    Header(bool),
    Length(u8),
    Data(u8),
}

impl<const N: usize> Default for Accumulator<N> {
    /// CRC check is enabled by default.
    fn default() -> Self {
        Self::new(true)
    }
}

impl<const N: usize> Accumulator<N> {
    /// Create a new accumulator.
    pub const fn new(crc: bool) -> Self {
        const {
            assert!(N >= 6, "Buffer too small");
            assert!(N <= u8::MAX as usize, "Buffer too large");
        };
        Accumulator {
            buf: [0; N],
            idx: 0,
            crc,
            state: FeedState::Empty,
        }
    }

    /// Reset the accumulator.
    fn reset(&mut self) {
        self.idx = 0;
        self.state = FeedState::Empty;
    }

    /// Appends data to the internal buffer and attempts to grab a [`RxFrame`].
    pub fn feed<'de, 'a>(&'de mut self, mut input: &'a [u8]) -> FeedResult<'de, 'a> {
        loop {
            if input.is_empty() {
                break FeedResult::Consumed;
            }

            let (&byte, remaining) = input.split_first().unwrap();
            input = remaining;
            break match self.feed_byte(byte) {
                // Consumed byte, still need more
                Ok(None) => continue,
                // Found errors while consummation
                Err(e) => {
                    // Reset the accumulator
                    self.reset();
                    FeedResult::Error(e, remaining)
                }
                // There is a frame ready to be grabbed
                Ok(Some(())) => {
                    let idx = self.idx;
                    self.reset();
                    // Deserialize the frame
                    match RxFrame::from_data_bytes(&self.buf[..idx]) {
                        Ok(frame) => FeedResult::Success(frame, remaining),
                        Err(e) => FeedResult::Error(e, remaining),
                    }
                }
            };
        }
    }

    /// Feeds a single byte to the internal buffer.
    fn feed_byte(&mut self, byte: u8) -> Result<Option<()>> {
        use FeedState::*;
        self.state = match self.state {
            Empty => {
                if byte == HEADER.to_be_bytes()[0] {
                    Header(false)
                } else {
                    return Err(Error::DeserializeBadHeader);
                }
            }
            Header(false) => {
                if byte == HEADER.to_be_bytes()[1] {
                    Header(true)
                } else {
                    return Err(Error::DeserializeBadHeader);
                }
            }
            Header(true) => {
                let min_len = if self.crc { 5 } else { 3 };
                if byte as usize >= min_len && byte as usize <= N {
                    Length(byte)
                } else {
                    return Err(Error::DeserializeBadLen);
                }
            }
            Length(length) => {
                *self
                    .buf
                    .get_mut(self.idx)
                    .ok_or(Error::AccumulateBufferFull)? = byte;
                self.idx += 1;
                Data(length - 1)
            }
            Data(length) => {
                *self
                    .buf
                    .get_mut(self.idx)
                    .ok_or(Error::AccumulateBufferFull)? = byte;
                self.idx += 1;
                Data(length - 1)
            }
        };

        if let Data(0) = self.state {
            if self.crc {
                let checksum = u16::from_le_bytes([self.buf[self.idx - 2], self.buf[self.idx - 1]]);
                if checksum != CRC.checksum(&self.buf[..self.idx - 2]) {
                    return Err(Error::DeserializeBadCrc);
                }
                self.idx -= 2;
            }
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Instruction;

    #[test]
    fn ack_crc() {
        let mut buf: Accumulator<64> = Accumulator::new(true);
        let ser = &[0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];

        if let FeedResult::Success(frame, remaining) = buf.feed(ser) {
            assert_eq!(
                frame.instr,
                Instruction::WriteWord {
                    addr: u16::from_be_bytes([b'O', b'K'])
                }
            );
            assert_eq!(remaining.len(), 4);
        } else {
            panic!()
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
    struct Demo {
        a: u16,
        b: bool,
        c: u32,
    }

    #[test]
    fn demo() {
        let mut buf: Accumulator<64> = Accumulator::new(false);
        let ser = &[
            0x5A, 0xA5, 12, 0x83, 0x12, 0x34, 4, 0xAA, 0xBB, 0x00, 0x01, 0xCC, 0xDD, 0xEE, 0xFF,
        ];

        if let FeedResult::Success(mut frame, remaining) = buf.feed(ser) {
            assert_eq!(
                frame.instr,
                Instruction::ReadWord {
                    addr: 0x1234,
                    len: 4
                }
            );

            assert_eq!(
                Demo {
                    a: 0xAABB,
                    b: true,
                    c: 0xCCDDEEFF
                },
                frame.take().unwrap()
            );
            assert_eq!(remaining.len(), 0);
        } else {
            panic!()
        }
    }

    #[test]
    fn double_demo() {
        let mut buf: Accumulator<64> = Accumulator::new(false);
        let ser = &[
            0x5A, 0xA5, 12, 0x83, 0x12, 0x34, 4, 0xAA, 0xBB, 0x00, 0x01, 0xCC, 0xDD, 0xEE, 0xFF,
            0x5A, 0xA5, 12, 0x83, 0x12, 0x34, 4, 0xBB, 0xAA, 0x00, 0x00, 0xFF, 0xEE, 0xDD, 0xCC,
        ];

        let (demo1, ser) = if let FeedResult::Success(mut frame, remaining) = buf.feed(ser) {
            (frame.take().unwrap(), remaining)
        } else {
            panic!()
        };

        assert_eq!(
            Demo {
                a: 0xAABB,
                b: true,
                c: 0xCCDDEEFF
            },
            demo1
        );

        let demo2 = if let FeedResult::Success(mut frame, remaining) = buf.feed(ser) {
            assert_eq!(remaining.len(), 0);
            frame.take().unwrap()
        } else {
            panic!()
        };

        assert_eq!(
            Demo {
                a: 0xBBAA,
                b: false,
                c: 0xFFEEDDCC,
            },
            demo2
        );
    }
}
