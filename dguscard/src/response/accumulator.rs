use crate::response::{Error, Response};
use crate::{CRC_LEN, Checksum, Crc, HEADER, MAX_CONTENT_LEN, MIN_CONTENT_LEN};

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
/// use dguscard::{response::{Response, Accumulator, FeedResult}, Crc};
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
/// let mut dgus_buf: Accumulator<128> = Accumulator::new(Crc::off());
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
///                     Response::Data(mut content) => {
///                         // Check where the data came from
///                         dbg!(content.addr(), content.wlen());
///                         // Take a MyData from the response content
///                         let data: MyData = content.take().unwrap();
///                     }
///                     Response::Ack => ()
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
pub struct Accumulator<const N: usize, K: Checksum = Crc> {
    buf: [u8; N],
    idx: usize,
    crc: Option<K>,
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

impl<const N: usize, K: Checksum> Accumulator<N, K> {
    /// Create a new accumulator.
    ///
    /// Pass [`Crc::on()`] or [`Crc::off()`] for `crc`, matching how the display
    /// is configured, or `Some(..)` to supply your own [`Checksum`].
    pub const fn new(crc: Option<K>) -> Self {
        const {
            assert!(N >= MIN_CONTENT_LEN, "Accumulator buffer size should be >= 3");
            assert!(N <= MAX_CONTENT_LEN, "Accumulator buffer size should be <= 255");
        };
        Accumulator {
            buf: [0; N],
            idx: 0,
            crc,
            state: FeedState::Empty,
        }
    }

    /// Reset the accumulator.
    const fn reset(&mut self) {
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
    fn feed_byte(&mut self, byte: u8) -> Result<Option<()>, Error> {
        self.state = match self.state {
            FeedState::Empty => {
                if byte == HEADER.to_be_bytes()[0] {
                    FeedState::Header(false)
                } else {
                    return Err(Error::BadHeader);
                }
            }
            FeedState::Header(false) => {
                if byte == HEADER.to_be_bytes()[1] {
                    FeedState::Header(true)
                } else {
                    return Err(Error::BadHeader);
                }
            }
            FeedState::Header(true) => {
                let trailer_len = if self.crc.is_some() { CRC_LEN } else { 0 };
                let min_len = (MIN_CONTENT_LEN + trailer_len) as u8;
                if byte < min_len {
                    return Err(Error::BadLen);
                }
                if byte as usize > N {
                    return Err(Error::FrameTooLarge);
                }
                FeedState::Length(byte)
            }
            FeedState::Length(length) | FeedState::Data(length) => {
                // `length` was bounded by `N` above, so `idx` stays within the buffer.
                self.buf[self.idx] = byte;
                self.idx += 1;
                FeedState::Data(length - 1)
            }
        };

        if let FeedState::Data(0) = self.state {
            if let Some(crc) = self.crc.as_mut() {
                let (content, trailer) = self.buf[..self.idx].split_at(self.idx - CRC_LEN);
                if u16::from_le_bytes([trailer[0], trailer[1]]) != crc.compute(content) {
                    return Err(Error::BadCrc);
                }
                self.idx -= CRC_LEN;
            }
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::Address;

    #[test]
    fn ack_crc() {
        let mut buf: Accumulator<64> = Accumulator::new(Crc::on());
        let ser = &[0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];

        if let FeedResult::Success(response, remaining) = buf.feed(ser) {
            let Response::Ack = response else { panic!() };
            assert_eq!(remaining.len(), 4);
        } else {
            panic!()
        }
    }

    #[test]
    fn bad_header() {
        let mut buf: Accumulator<64> = Accumulator::new(Crc::off());

        let FeedResult::Error(e, remaining) = buf.feed(&[0x11, 0x22, 0x33]) else {
            panic!("Expected an error")
        };
        assert_eq!(e, Error::BadHeader);
        // The rest of the input is handed back so the caller can resync.
        assert_eq!(remaining, &[0x22, 0x33]);
    }

    #[test]
    fn frame_too_large() {
        let mut buf: Accumulator<8> = Accumulator::new(Crc::off());

        let FeedResult::Error(e, remaining) = buf.feed(&[0x5A, 0xA5, 9, 0x83]) else {
            panic!("Expected an error")
        };
        assert_eq!(e, Error::FrameTooLarge);
        assert_eq!(remaining, &[0x83]);
    }

    #[test]
    fn minimal_buffer_without_crc() {
        // An ACK is the shortest frame there is, and without a checksum it
        // needs only three bytes of content. The previous `N >= 5` bound
        // rejected this at compile time.
        let mut buf: Accumulator<3> = Accumulator::new(Crc::off());

        let FeedResult::Success(Response::Ack, remaining) = buf.feed(&[0x5A, 0xA5, 3, 0x82, b'O', b'K']) else {
            panic!("Expected an ACK")
        };
        assert!(remaining.is_empty());
    }

    #[test]
    fn frame_exactly_fills_buffer() {
        let mut buf: Accumulator<8> = Accumulator::new(Crc::off());
        let ser = &[0x5A, 0xA5, 8, 0x83, 0x12, 0x34, 2, 0xAA, 0xBB, 0xCC, 0xDD];

        let FeedResult::Success(response, remaining) = buf.feed(ser) else {
            panic!("Expected a response")
        };
        let Response::Data(mut data) = response else {
            panic!("Expected Data response")
        };
        assert_eq!(data.addr(), Address::Word(0x1234));
        assert_eq!(data.wlen(), 2);
        assert_eq!(data.take::<u32>().unwrap(), 0xAABB_CCDD);
        assert!(remaining.is_empty());
    }

    #[test]
    fn maximal_frame() {
        let mut buf: Accumulator<{ MAX_CONTENT_LEN }> = Accumulator::new(Crc::off());

        // A frame whose length byte is saturated: 255 content bytes, the most a
        // DGUS frame can describe.
        let mut ser = [0u8; crate::MAX_FRAME_LEN];
        ser[0] = 0x5A;
        ser[1] = 0xA5;
        ser[2] = MAX_CONTENT_LEN as u8;
        ser[3] = 0x83;
        ser[4] = 0x12;
        ser[5] = 0x34;
        ser[6] = 125;

        let FeedResult::Success(response, remaining) = buf.feed(&ser) else {
            panic!("Expected a response")
        };
        let Response::Data(data) = response else {
            panic!("Expected Data response")
        };
        assert_eq!(data.addr(), Address::Word(0x1234));
        assert_eq!(data.wlen(), 125);
        assert_eq!(data.len(), MAX_CONTENT_LEN - 4);
        assert!(remaining.is_empty());
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Demo {
        a: u16,
        b: bool,
        c: u32,
    }

    #[test]
    fn demo() {
        let mut buf: Accumulator<64> = Accumulator::new(Crc::off());
        let ser = &[
            0x5A, 0xA5, 12, 0x83, 0x12, 0x34, 4, 0xAA, 0xBB, 0x00, 0x01, 0xCC, 0xDD, 0xEE, 0xFF,
        ];

        if let FeedResult::Success(response, remaining) = buf.feed(ser) {
            let Response::Data(mut data) = response else {
                panic!("Expected Data response");
            };

            assert_eq!(data.addr(), Address::Word(0x1234));
            assert_eq!(data.wlen(), 4);

            assert_eq!(
                Demo {
                    a: 0xAABB,
                    b: true,
                    c: 0xCCDD_EEFF
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
        let mut buf: Accumulator<64> = Accumulator::new(Crc::off());
        let ser = &[
            0x5A, 0xA5, 12, 0x83, 0x12, 0x34, 4, 0xAA, 0xBB, 0x00, 0x01, 0xCC, 0xDD, 0xEE, 0xFF, 0x5A, 0xA5, 12, 0x83,
            0x12, 0x34, 4, 0xBB, 0xAA, 0x00, 0x00, 0xFF, 0xEE, 0xDD, 0xCC,
        ];

        let FeedResult::Success(response, remaining) = buf.feed(ser) else {
            panic!("Expected Success");
        };

        let Response::Data(mut data) = response else {
            panic!("Expected Data response");
        };

        assert_eq!(data.addr(), Address::Word(0x1234));
        assert_eq!(data.wlen(), 4);

        assert_eq!(
            Demo {
                a: 0xAABB,
                b: true,
                c: 0xCCDD_EEFF
            },
            data.take().unwrap()
        );

        let FeedResult::Success(response, remaining) = buf.feed(remaining) else {
            panic!("Expected Success");
        };

        let Response::Data(mut data) = response else {
            panic!("Expected Data response");
        };

        assert_eq!(data.addr(), Address::Word(0x1234));
        assert_eq!(data.wlen(), 4);

        assert_eq!(
            Demo {
                a: 0xBBAA,
                b: false,
                c: 0xFFEE_DDCC,
            },
            data.take().unwrap()
        );
        assert!(remaining.is_empty());
    }
}
