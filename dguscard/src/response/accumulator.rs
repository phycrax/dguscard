use crate::response::{Error, Response};
use crate::{CRC_LEN, Checksum, Crc, HEADER, MIN_CONTENT_LEN};

/// An [`Accumulator`] that owns a `[u8; N]` buffer.
///
/// Construct with [`ArrayAccumulator::new`].
pub type ArrayAccumulator<const N: usize, K = Crc> = Accumulator<[u8; N], K>;

/// An [`Accumulator`] that borrows its buffer.
///
/// Construct with [`SliceAccumulator::new`].
pub type SliceAccumulator<'a, K = Crc> = Accumulator<&'a mut [u8], K>;

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
/// use dguscard::{response::{Response, ArrayAccumulator, FeedResult}, Crc};
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
/// let mut dgus_buf = ArrayAccumulator::<128>::new(Crc::off());
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
/// Use a borrowed buffer instead.
///
/// ```rust
/// use dguscard::{response::{FeedResult, Response, SliceAccumulator}, Crc};
///
/// let mut buf = [0u8; 128];
/// let mut dgus_buf = SliceAccumulator::new(&mut buf, Crc::off());
///
/// let FeedResult::Success(Response::Ack, remaining) =
///     dgus_buf.feed(&[0x5A, 0xA5, 3, 0x82, b'O', b'K'])
/// else {
///     panic!("expected an ACK")
/// };
/// ```
///
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Accumulator<B: AsRef<[u8]> + AsMut<[u8]>, K: Checksum = Crc> {
    buf: B,
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

const fn min_content_len(crc: bool) -> usize {
    MIN_CONTENT_LEN + if crc { CRC_LEN } else { 0 }
}

impl<const N: usize, K: Checksum> ArrayAccumulator<N, K> {
    /// Create a new accumulator backed by an owned `[u8; N]`.
    ///
    /// Pass [`Crc::on()`] or [`Crc::off()`] for `crc`, matching how the display
    /// is configured, or `Some(..)` to supply your own [`Checksum`].
    pub const fn new(crc: Option<K>) -> Self {
        Accumulator {
            buf: [0; N],
            idx: 0,
            crc,
            state: FeedState::Empty,
        }
    }
}

impl<'a, K: Checksum> SliceAccumulator<'a, K> {
    /// Create a new accumulator backed by a borrowed buffer.
    ///
    /// Pass [`Crc::on()`] or [`Crc::off()`] for `crc`, matching how the display
    /// is configured, or `Some(..)` to supply your own [`Checksum`].
    pub const fn new(buf: &'a mut [u8], crc: Option<K>) -> Self {
        Accumulator {
            buf,
            idx: 0,
            crc,
            state: FeedState::Empty,
        }
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>, K: Checksum> Accumulator<B, K> {
    /// Create a new accumulator.
    ///
    /// It should rarely be necessary to call this directly:
    /// [`ArrayAccumulator::new`] and [`SliceAccumulator::new`] cover the two common cases.
    ///
    /// Pass [`Crc::on()`] or [`Crc::off()`] for `crc`, matching how the display
    /// is configured, or `Some(..)` to supply your own [`Checksum`].
    pub const fn with_buffer(buf: B, crc: Option<K>) -> Self {
        Accumulator {
            buf,
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
            let Some((&byte, remaining)) = input.split_first() else {
                break FeedResult::Consumed;
            };

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
                    match Response::from_content_bytes(&self.buf.as_ref()[..idx]) {
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
                if (byte as usize) < min_content_len(self.crc.is_some()) {
                    return Err(Error::BadLen);
                }
                if byte as usize > self.buf.as_ref().len() {
                    return Err(Error::FrameTooLarge);
                }
                FeedState::Length(byte)
            }
            FeedState::Length(length) | FeedState::Data(length) => {
                // `length` was bounded by the buffer length above, so `idx` stays within it.
                self.buf.as_mut()[self.idx] = byte;
                self.idx += 1;
                FeedState::Data(length - 1)
            }
        };

        if let FeedState::Data(0) = self.state {
            if let Some(crc) = self.crc.as_mut() {
                let (content, trailer) = self.buf.as_ref()[..self.idx].split_at(self.idx - CRC_LEN);
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
    use crate::{Address, FRAME_OVERHEAD, MAX_CONTENT_LEN};

    const MAX_FRAME_LEN: usize = u8::MAX as usize + FRAME_OVERHEAD;

    #[test]
    fn ack_crc() {
        let mut buf = ArrayAccumulator::<64>::new(Crc::on());
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
        let mut buf = ArrayAccumulator::<64>::new(Crc::off());

        let FeedResult::Error(e, remaining) = buf.feed(&[0x11, 0x22, 0x33]) else {
            panic!("Expected an error")
        };
        assert_eq!(e, Error::BadHeader);
        // The rest of the input is handed back so the caller can resync.
        assert_eq!(remaining, &[0x22, 0x33]);
    }

    #[test]
    fn frame_too_large() {
        let mut buf = ArrayAccumulator::<8>::new(Crc::off());

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
        let mut buf = ArrayAccumulator::<3>::new(Crc::off());

        let FeedResult::Success(Response::Ack, remaining) = buf.feed(&[0x5A, 0xA5, 3, 0x82, b'O', b'K']) else {
            panic!("Expected an ACK")
        };
        assert!(remaining.is_empty());
    }

    #[test]
    fn frame_exactly_fills_buffer() {
        let mut buf = ArrayAccumulator::<8>::new(Crc::off());
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
        let mut buf = ArrayAccumulator::<MAX_CONTENT_LEN>::new(Crc::off());

        // A frame whose length byte is saturated: 255 content bytes, the most a
        // DGUS frame can describe.
        let mut ser = [0u8; MAX_FRAME_LEN];
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
        let mut buf = ArrayAccumulator::<64>::new(Crc::off());
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
        let mut buf = ArrayAccumulator::<64>::new(Crc::off());
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

    #[test]
    fn slice_ack_crc() {
        let mut backing = [0u8; 64];
        let mut buf = SliceAccumulator::new(&mut backing, Crc::on());
        let ser = &[0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];

        let FeedResult::Success(Response::Ack, remaining) = buf.feed(ser) else {
            panic!("Expected an ACK")
        };
        assert_eq!(remaining.len(), 4);
    }

    #[test]
    fn slice_frame_too_large() {
        let mut backing = [0u8; 8];
        let mut buf = SliceAccumulator::new(&mut backing, Crc::off());

        let FeedResult::Error(e, remaining) = buf.feed(&[0x5A, 0xA5, 9, 0x83]) else {
            panic!("Expected an error")
        };
        assert_eq!(e, Error::FrameTooLarge);
        assert_eq!(remaining, &[0x83]);
    }

    #[test]
    fn slice_minimal_buffer() {
        let mut backing = [0u8; MIN_CONTENT_LEN];
        let mut buf = SliceAccumulator::new(&mut backing, Crc::off());

        let FeedResult::Success(Response::Ack, remaining) = buf.feed(&[0x5A, 0xA5, 3, 0x82, b'O', b'K']) else {
            panic!("Expected an ACK")
        };
        assert!(remaining.is_empty());
    }

    #[test]
    fn slice_too_small_rejects_frames() {
        // Construction accepts an undersized buffer; the length check in `feed`
        // rejects every frame instead, since none can fit.
        let mut backing = [0u8; MIN_CONTENT_LEN - 1];
        let mut buf = SliceAccumulator::new(&mut backing, Crc::off());

        let FeedResult::Error(e, remaining) = buf.feed(&[0x5A, 0xA5, 3, 0x82, b'O', b'K']) else {
            panic!("Expected an error")
        };
        assert_eq!(e, Error::FrameTooLarge);
        assert_eq!(remaining, &[0x82, b'O', b'K']);
    }

    #[test]
    fn minimal_buffer_with_crc() {
        // With a checksum the trailer is accumulated alongside the content, so
        // the shortest ACK declares five bytes and needs five bytes of buffer.
        let mut backing = [0u8; MIN_CONTENT_LEN + CRC_LEN];
        let mut buf = SliceAccumulator::new(&mut backing, Crc::on());

        let FeedResult::Success(Response::Ack, remaining) = buf.feed(&[0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF])
        else {
            panic!("Expected an ACK")
        };
        assert!(remaining.is_empty());
    }

    #[test]
    fn slice_too_small_for_crc_rejects_frames() {
        // Four bytes clears the no-CRC floor but can never hold a checksummed frame.
        let mut backing = [0u8; MIN_CONTENT_LEN + CRC_LEN - 1];
        let mut buf = SliceAccumulator::new(&mut backing, Crc::on());

        let FeedResult::Error(e, remaining) = buf.feed(&[0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF]) else {
            panic!("Expected an error")
        };
        assert_eq!(e, Error::FrameTooLarge);
        assert_eq!(remaining, &[0x82, b'O', b'K', 0xA5, 0xEF]);
    }

    #[test]
    fn array_too_small_for_crc_rejects_frames() {
        let mut buf = ArrayAccumulator::<{ MIN_CONTENT_LEN + CRC_LEN - 1 }>::new(Crc::on());

        let FeedResult::Error(e, remaining) = buf.feed(&[0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF]) else {
            panic!("Expected an error")
        };
        assert_eq!(e, Error::FrameTooLarge);
        assert_eq!(remaining, &[0x82, b'O', b'K', 0xA5, 0xEF]);
    }

    #[test]
    fn slice_split_feed() {
        // The frame arrives in two chunks, so accumulation state has to survive
        // the boundary between the two `feed` calls.
        let mut backing = [0u8; 64];
        let mut buf = SliceAccumulator::new(&mut backing, Crc::off());
        let ser = &[
            0x5A, 0xA5, 12, 0x83, 0x12, 0x34, 4, 0xAA, 0xBB, 0x00, 0x01, 0xCC, 0xDD, 0xEE, 0xFF,
        ];
        let (head, tail) = ser.split_at(7);

        let FeedResult::Consumed = buf.feed(head) else {
            panic!("Expected the first chunk to be consumed")
        };

        let FeedResult::Success(Response::Data(mut data), remaining) = buf.feed(tail) else {
            panic!("Expected Data response")
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
        assert!(remaining.is_empty());
    }
}
