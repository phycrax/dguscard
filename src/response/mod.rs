//! Response parser

mod accumulator;
mod deserializer;

pub use self::accumulator::{Accumulator, FeedResult};

use self::deserializer::Deserializer;
use crate::{
    command::{Command, Curve, Dword, Read, Register, Word, Write},
    Error::*,
    Result, CRC, HEADER,
};
use serde::Deserialize;

/// [`serde`] compatible deserializer wrapping over raw data part of the response
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Content<'de> {
    deserializer: Deserializer<'de>,
}

impl<'de> Content<'de> {
    /// Removes a `T` from content and returns it.
    pub fn take<T: Deserialize<'de>>(&mut self) -> Result<T> {
        T::deserialize(&mut self.deserializer)
    }

    /// Returns the number of remaining bytes in the content.
    pub fn len(&self) -> usize {
        self.deserializer.input.len()
    }

    /// Returns true if the content does not contain any remaining bytes.
    pub fn is_empty(&self) -> bool {
        self.deserializer.input.is_empty()
    }
}

/// Response parser
///
/// # Examples
///
/// ```rust
/// use dguscard::response::Response;
/// # use std::io::Read;
/// #[derive(serde::Deserialize)]
/// struct MyData {
///     byte: u8,
///     word: u16,
///     dword: u32,
///     float: f32,
///     double: f64,
/// }
///
/// let mut uart =
/// # std::collections::VecDeque::from([
/// # 0x5A, 0xA5, 27, 0x83, 0x12, 0x34, 12,
/// # 0x00, // MyData.byte
/// # 0x11, 0x11, // MyData.word
/// # 0x22, 0x22, 0x22, 0x22, // MyData.dword
/// # 0x33, 0x33, 0x33, 0x33, // MyData.float
/// # 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, // MyData.double
/// # 0x12, 0x34, 0x56, 0x78 // single u32
/// # ]);
/// // Backing buffer for the UART.
/// let buf = &mut [0u8; 50];
/// // Read a full response from UART with something like read_until_idle().
/// // If this is not possible, take a look at the `Accumulator`.
/// let _ = uart.read(buf).unwrap();
/// // Look for a full response within the buffer.
/// let mut response = Response::from_bytes(buf, false).unwrap();
/// // Handle the response.
/// match response {
///     Response::WordData{cmd, mut content} => {
///         // Check response command
///         dbg!(cmd);
///         // Take a MyData from the response content
///         let my_data: MyData = content.take().unwrap();
///         // Take an u32 from the response content
///         let dword_int: u32 = content.take().unwrap();
///     }
///     _ => (),
/// }
/// ```
///
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Response<'de> {
    /// Response for [`Register<Write>`] request
    RegisterAck,
    /// Response for [`Word<Write>`] request
    WordAck,
    /// Response for [`Dword<Write>`] request
    DwordAck,
    /// Response for [`Curve`] request
    CurveAck,

    /// Response for [`Register<Read>`] request
    RegisterData {
        /// Command
        cmd: Register<Read>,
        /// Content
        content: Content<'de>,
    },
    /// Response for [`Word<Read>`] request
    WordData {
        /// Command
        cmd: Word<Read>,
        /// Content
        content: Content<'de>,
    },
    /// Response for [`Dword<Read>`] request
    DwordData {
        /// Command
        cmd: Dword<Read>,
        /// Content
        content: Content<'de>,
    },
}

impl<'de> Response<'de> {
    /// Looks for a response within a byte slice.
    /// The unused portion (if any) of the byte slice is not returned.
    /// The byte slice is expected to contain full response, including header, length, and CRC if enabled.
    pub fn from_bytes(input: &'de [u8], crc: bool) -> Result<Self> {
        let (response, _) = Self::take_from_bytes(input, crc)?;
        Ok(response)
    }

    /// Looks for a response within a byte slice.
    /// The unused portion (if any) of the byte slice is returned for further usage.
    /// The byte slice is expected to contain full response, including header, length, and CRC if enabled.
    pub fn take_from_bytes(input: &'de [u8], crc: bool) -> Result<(Self, &'de [u8])> {
        let (input, rest) = Self::extract_content_bytes(input, crc)?;
        Ok((Self::from_content_bytes(input)?, rest))
    }

    /// Looks for a response within a byte slice.
    /// The unused portion (if any) of the byte slice is not returned.
    /// The data byte slice is expected to contain command and data section of the response,
    /// i.e. excluding header, length, and CRC if enabled.
    /// Intended to be used with an Accumulator.
    pub fn from_content_bytes(input: &'de [u8]) -> Result<Self> {
        let mut deserializer = Deserializer { input };
        // Strip command code from input
        let opcode = u8::deserialize(&mut deserializer)?;
        use Response::*;
        // Is it ACK?
        if opcode % 2 == 0 {
            let response = match opcode {
                Register::<Write>::CMD => RegisterAck,
                Word::<Write>::CMD => WordAck,
                Dword::<Write>::CMD => DwordAck,
                Curve::CMD => CurveAck,
                _ => return Err(ResponseUnknownCmd),
            };
            // Verify ACK bytes
            if u16::deserialize(&mut deserializer)? != u16::from_be_bytes([b'O', b'K']) {
                return Err(ResponseBadAck);
            }
            Ok(response)
        }
        // Or is it data?
        else {
            let response = match opcode {
                Register::<Read>::CMD => RegisterData {
                    cmd: Register::deserialize(&mut deserializer)?,
                    content: Content { deserializer },
                },
                Word::<Read>::CMD => WordData {
                    cmd: Word::deserialize(&mut deserializer)?,
                    content: Content { deserializer },
                },
                Dword::<Read>::CMD => DwordData {
                    cmd: Dword::deserialize(&mut deserializer)?,
                    content: Content { deserializer },
                },
                _ => return Err(ResponseUnknownCmd),
            };
            Ok(response)
        }
    }

    /// Extracts the command+data part of the response from a byte slice.
    /// The unused portion (if any) of the byte slice is returned for further usage.
    /// The byte slice is expected to contain full DGUS response, including header, length, and CRC if enabled.
    fn extract_content_bytes(input: &'de [u8], crc: bool) -> Result<(&'de [u8], &'de [u8])> {
        // Strip header from input
        let input = input
            .strip_prefix(&u16::to_be_bytes(HEADER))
            .ok_or(ResponseBadHeader)?;

        // Strip length from input
        let (len, input) = input.split_first().ok_or(DeserializeUnexpectedEnd)?;
        let len = *len as usize;
        let min_len = if crc { 5 } else { 3 };
        if len < min_len {
            return Err(ResponseBadLen);
        }

        // Split input with the length
        let (input, rest) = input.split_at_checked(len).ok_or(ResponseTooLarge)?;

        // Strip CRC from input
        let input = if crc {
            let (input, crc) = input.split_last_chunk().ok_or(DeserializeUnexpectedEnd)?;
            if u16::from_le_bytes(*crc) != CRC.checksum(input) {
                return Err(ResponseBadCrc);
            }
            input
        } else {
            input
        };

        Ok((input, rest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_crc() {
        let input = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 1, 2, 3, 4];
        let (response, rest) = Response::take_from_bytes(&input, true).unwrap();
        let Response::WordAck = response else {
            panic!()
        };
        assert_eq!(rest, &[1, 2, 3, 4]);
    }
}
