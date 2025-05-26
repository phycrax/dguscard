use crate::{response::Response, Error, Result, CRC, HEADER};

/// An accumulator used to collect chunked response
///
/// This is often useful when you receive "parts" of the response at a time, for example when draining
/// a serial port buffer that may not contain an entire uninterrupted response.
///
/// # Examples
///
/// Collect a response by reading chunks.
///
/// ```rust
/// use dguscard::response::{Response, Accumulator, FeedResult};
/// # use std::io::Read;
///
/// #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
/// struct MyData {
///     a: u16,
///     b: bool,
///     c: u32,
/// }
///
/// let mut uart =
/// # &[0x5A, 0xA5, 12, 0x83, 0x12, 0x34, 4, 0xAA, 0xBB, 0x00, 0x01, 0xCC, 0xDD, 0xEE, 0xFF][..];
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
///                 // Handle error here.
///                 dbg!(error);
///                 // Move the window
///                 remaining
///             },
///             FeedResult::Success(response, remaining) => {
///                 // Handle response here.  
///                 match response {
///                     Response::WordData { cmd, mut data } => {
///                         // Check response command
///                         dbg!(cmd);
///                         // Take a MyData from the response data
///                         let data: MyData = data.take().unwrap();
///                     }
///                     _ => ()
///                 }
///                 // Move the window
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

/// The result of feeding the accumulator
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FeedResult<'de, 'a> {
    /// Consumed all data, still pending.
    Consumed,
    /// Accumulation failed. Contains remaining section of input, if any.
    Error(Error, &'a [u8]),
    /// Accumulation successful. Contains a response and remaining section of input, if any.
    Success(Response<'de>, &'a [u8]),
}

/// The internal state of feeding the accumulator.
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
            assert!(N >= 5, "Accumulator buffer size should be >= 5");
            assert!(
                N <= u8::MAX as usize,
                "Accumulator buffer size should be <= 256"
            );
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

    /// Appends data to the internal buffer and attempts to grab a [`Response`].
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
                // There is a response ready to be grabbed
                Ok(Some(())) => {
                    let idx = self.idx;
                    self.reset();
                    // Construct the response
                    match Response::from_content_bytes(&self.buf[..idx]) {
                        Ok(response) => FeedResult::Success(response, remaining),
                        Err(e) => FeedResult::Error(e, remaining),
                    }
                }
            };
        }
    }

    /// Feeds a single byte to the internal buffer.
    fn feed_byte(&mut self, byte: u8) -> Result<Option<()>> {
        use Error::*;
        use FeedState::*;
        self.state = match self.state {
            Empty => {
                if byte == HEADER.to_be_bytes()[0] {
                    Header(false)
                } else {
                    return Err(ResponseBadHeader);
                }
            }
            Header(false) => {
                if byte == HEADER.to_be_bytes()[1] {
                    Header(true)
                } else {
                    return Err(ResponseBadHeader);
                }
            }
            Header(true) => {
                let min_len = if self.crc { 5 } else { 3 };
                if byte < min_len {
                    return Err(ResponseBadLen);
                }
                if byte as usize >= N {
                    return Err(ResponseTooLarge);
                }
                Length(byte)
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
                    return Err(ResponseBadCrc);
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
    use crate::command::{Read, Word};

    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn ack_crc() {
        let mut buf: Accumulator<64> = Accumulator::new(true);
        let ser = &[0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];

        if let FeedResult::Success(response, remaining) = buf.feed(ser) {
            let Response::WordAck = response else {
                panic!()
            };
            assert_eq!(remaining.len(), 4);
        } else {
            panic!()
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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

        if let FeedResult::Success(response, remaining) = buf.feed(ser) {
            let Response::WordData { cmd, mut data } = response else {
                panic!("Expected ReadWord response");
            };

            assert_eq!(
                cmd,
                Word {
                    addr: 0x1234,
                    cmd: Read { wlen: 4 }
                }
            );

            assert_eq!(
                Demo {
                    a: 0xAABB,
                    b: true,
                    c: 0xCCDDEEFF
                },
                data.take().unwrap()
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

        let FeedResult::Success(response, remaining) = buf.feed(ser) else {
            panic!("Expected Success");
        };

        let Response::WordData { cmd, mut data } = response else {
            panic!("Expected ReadWord response");
        };

        assert_eq!(
            cmd,
            Word {
                addr: 0x1234,
                cmd: Read { wlen: 4 }
            }
        );

        assert_eq!(
            Demo {
                a: 0xAABB,
                b: true,
                c: 0xCCDDEEFF
            },
            data.take().unwrap()
        );

        let FeedResult::Success(response, remaining) = buf.feed(remaining) else {
            panic!("Expected Success");
        };

        let Response::WordData { cmd, mut data } = response else {
            panic!("Expected ReadWord response");
        };

        assert_eq!(
            cmd,
            Word {
                addr: 0x1234,
                cmd: Read { wlen: 4 }
            }
        );

        assert_eq!(
            Demo {
                a: 0xBBAA,
                b: false,
                c: 0xFFEEDDCC,
            },
            data.take().unwrap()
        );
        assert!(remaining.is_empty());
    }
}
