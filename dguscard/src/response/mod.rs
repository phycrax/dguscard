//! Response parser

mod accumulator;
mod deserializer;

pub use self::accumulator::{Accumulator, FeedResult};

use self::deserializer::Deserializer;
use crate::{Address, CRC, Error, HEADER, Result};
use serde::Deserialize;

/// A data response: where it came from, how long it is, and a [`serde`]
/// compatible deserializer over its not-yet-taken data.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Content<'de> {
    addr: Address,
    wlen: u8,
    deserializer: Deserializer<'de>,
}

impl<'de> Content<'de> {
    /// Creates a [`Content`] from an address, a word length, and the raw data
    /// bytes, without any framing.
    ///
    /// The bytes are the data part of a response, i.e. what remains after the
    /// command, address, and word length have been stripped. Parsing a response
    /// off the wire produces a [`Content`] already; this is for reconstructing
    /// one from parts captured elsewhere, e.g. copied out of an [`Accumulator`]
    /// by a driver and handed across a channel.
    pub const fn new(addr: Address, wlen: u8, data: &'de [u8]) -> Self {
        Self {
            addr,
            wlen,
            deserializer: Deserializer { input: data },
        }
    }

    /// Returns the address the data came from.
    pub const fn addr(&self) -> Address {
        self.addr
    }

    /// Returns the word length reported by the response.
    pub const fn wlen(&self) -> u8 {
        self.wlen
    }

    /// Removes a `T` from content and returns it.
    pub fn take<T: Deserialize<'de>>(&mut self) -> Result<T> {
        T::deserialize(&mut self.deserializer)
    }

    /// Returns the number of remaining bytes in the content.
    pub const fn len(&self) -> usize {
        self.deserializer.input.len()
    }

    /// Returns true if the content does not contain any remaining bytes.
    pub const fn is_empty(&self) -> bool {
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
///     Response::Data(mut content) => {
///         // Check where the data came from
///         dbg!(content.addr(), content.wlen());
///         // Take a MyData from the response content
///         let my_data: MyData = content.take().unwrap();
///         // Take an u32 from the response content
///         let dword_int: u32 = content.take().unwrap();
///     }
///     Response::Ack => (),
/// }
/// ```
///
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Response<'de> {
    /// Response for a [`Write`][crate::request::Write] request.
    Ack,

    /// Response for a [`Read`][crate::request::Read] request.
    Data(Content<'de>),
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
        // Is it ACK?
        if opcode % 2 == 0 {
            // Validate the write command code
            match opcode {
                Address::REGISTER | Address::WORD | Address::CURVE | Address::DWORD => {}
                _ => return Err(Error::ResponseUnknownCmd),
            }
            // Verify ACK bytes
            if u16::deserialize(&mut deserializer)? != u16::from_be_bytes(*b"OK") {
                return Err(Error::ResponseBadAck);
            }
            Ok(Response::Ack)
        }
        // Or is it data?
        else {
            // Strip address from input
            let addr = match opcode - 1 {
                Address::REGISTER => Address::Register(
                    u8::deserialize(&mut deserializer)?,
                    u8::deserialize(&mut deserializer)?,
                ),
                Address::WORD => Address::Word(u16::deserialize(&mut deserializer)?),
                Address::DWORD => Address::Dword(u32::deserialize(&mut deserializer)?),
                _ => return Err(Error::ResponseUnknownCmd),
            };
            // Strip word length from input
            let wlen = u8::deserialize(&mut deserializer)?;
            Ok(Response::Data(Content {
                addr,
                wlen,
                deserializer,
            }))
        }
    }

    /// Extracts the command+data part of the response from a byte slice.
    /// The unused portion (if any) of the byte slice is returned for further usage.
    /// The byte slice is expected to contain full DGUS response, including header, length, and CRC if enabled.
    fn extract_content_bytes(input: &'de [u8], crc: bool) -> Result<(&'de [u8], &'de [u8])> {
        // Strip header from input
        let input = input
            .strip_prefix(&u16::to_be_bytes(HEADER))
            .ok_or(Error::ResponseBadHeader)?;

        // Strip length from input
        let (len, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
        let len = *len as usize;
        let min_len = if crc { 5 } else { 3 };
        if len < min_len {
            return Err(Error::ResponseBadLen);
        }

        // Split input with the length
        let (input, rest) = input.split_at_checked(len).ok_or(Error::ResponseTooLarge)?;

        // Strip CRC from input
        let input = if crc {
            let (input, crc) = input
                .split_last_chunk()
                .ok_or(Error::DeserializeUnexpectedEnd)?;
            if u16::from_le_bytes(*crc) != CRC.checksum(input) {
                return Err(Error::ResponseBadCrc);
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
        let Response::Ack = response else { panic!() };
        assert_eq!(rest, &[1, 2, 3, 4]);
    }

    #[test]
    fn content_new() {
        // Data part only: a u16 followed by a u32, big-endian.
        let bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let mut content = Content::new(Address::Word(0x1234), 3, &bytes);
        assert_eq!(content.addr(), Address::Word(0x1234));
        assert_eq!(content.wlen(), 3);
        assert_eq!(content.len(), 6);
        assert_eq!(content.take::<u16>().unwrap(), 0xAABB);
        assert_eq!(content.take::<u32>().unwrap(), 0xCCDD_EEFF);
        assert!(content.is_empty());
    }

    #[test]
    fn data_content_register_and_dword() {
        let input = [0x5A, 0xA5, 4, 0x81, 0x00, 0x04, 1, 0xAA, 0xBB];
        let Response::Data(content) = Response::from_bytes(&input, false).unwrap() else {
            panic!("Expected Data response");
        };
        assert_eq!(content.addr(), Address::Register(0x00, 0x04));
        assert_eq!(content.wlen(), 1);

        let input = [0x5A, 0xA5, 6, 0x87, 0x00, 0x01, 0x02, 0x03, 1, 0xAA, 0xBB];
        let Response::Data(content) = Response::from_bytes(&input, false).unwrap() else {
            panic!("Expected Data response");
        };
        assert_eq!(content.addr(), Address::Dword(0x0001_0203));
        assert_eq!(content.wlen(), 1);
    }

    #[test]
    fn unknown_cmd() {
        // The curve buffer is write only, so there is no 0x85 response.
        assert!(matches!(
            Response::from_content_bytes(&[0x85, 0x00, 1, 0xAA, 0xBB]),
            Err(Error::ResponseUnknownCmd)
        ));
    }
}
