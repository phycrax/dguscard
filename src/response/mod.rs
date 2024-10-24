//! Response parser/deserializer/accumulator

mod accumulator;
mod deserializer;

pub use self::accumulator::{Accumulator, FeedResult};

use self::deserializer::Deserializer;
use crate::{
    Curve, Dword, Error::*, Instruction, Read, Register, Result, Word, Write, CRC, HEADER,
};
use serde::Deserialize;

/// Response data slice with deserialization capability
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ResponseData<'de> {
    deserializer: Deserializer<'de>,
}

impl<'de> ResponseData<'de> {
    /// Removes a `T` from the slice and returns it.
    pub fn take<T: Deserialize<'de>>(&mut self) -> Result<T> {
        T::deserialize(&mut self.deserializer)
    }

    /// Returns the number of remaining bytes in the response.
    pub fn len(&self) -> usize {
        self.deserializer.input.len()
    }

    /// Returns true if the slice does not contain any remaining bytes.
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
///     ah: u8,
///     al: u8,
///     b: u16,
///     c: u32,
/// }
///
/// let mut uart =
/// # std::collections::VecDeque::from([
/// # 0x5A, 0xA5, 16, 0x83, 0x12, 0x34, 4, 0xAA, 0xBB, 0x11, 0x11,
/// # 0x22, 0x22, 0x22, 0x22, 0x33, 0x33, 0x33, 0x33]);
/// // Backing buffer for the UART.
/// let buf = &mut [0u8; 50];
/// // Read a response from UART.
/// let _ = uart.read(buf).unwrap();
/// // Look for a response within the buffer.
/// let mut response = Response::from_bytes(buf, false).unwrap();
/// // Do something with the response.
/// match response {
///     Response::WordData{instr, mut data} => {
///         // Check response instruction
///         dbg!(instr);
///         // Take a MyData from the response data
///         let my_data: MyData = data.take().unwrap();
///         // Take an u32 from the response data
///         let integer: u32 = data.take().unwrap();
///     }
///     _ => (),
/// }
/// ```
///
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Response<'de> {
    /// ACK Response for Register<Write> request
    RegisterAck,
    /// ACK Response for Word<Write> request
    WordAck,
    /// ACK Response for Dword<Write> request
    DwordAck,
    /// ACK Response for Curve request
    CurveAck,

    /// Data Response for Register<Read> request
    RegisterData {
        /// Instruction
        instr: Register<Read>,
        /// Data
        data: ResponseData<'de>,
    },
    /// Data Response for Word<Read> request
    WordData {
        /// Instruction
        instr: Word<Read>,
        /// Data
        data: ResponseData<'de>,
    },
    /// Data Response for Dword<Read> request
    DwordData {
        /// Instruction
        instr: Dword<Read>,
        /// Data
        data: ResponseData<'de>,
    },
}

impl<'de> Response<'de> {
    /// Looks for a response within a byte slice.
    /// The unused portion (if any) of the byte slice is not returned.
    /// The byte slice is expected to contain full DGUS response, including header, length, and CRC if enabled.
    pub fn from_bytes(input: &'de [u8], crc: bool) -> Result<Self> {
        let (response, _) = Self::take_from_bytes(input, crc)?;
        Ok(response)
    }

    /// Looks for a response within a byte slice.
    /// The unused portion (if any) of the byte slice is returned for further usage.
    /// The byte slice is expected to contain full DGUS response, including header, length, and CRC if enabled.
    pub fn take_from_bytes(input: &'de [u8], crc: bool) -> Result<(Self, &'de [u8])> {
        let (input, rest) = Self::extract_data_bytes(input, crc)?;
        Ok((Self::from_content_bytes(input)?, rest))
    }

    /// Looks for a response within a byte slice.
    /// The unused portion (if any) of the byte slice is not returned.
    /// The data byte slice is expected to contain instruction and data section of the DGUS response,
    /// i.e. excluding header, length, and CRC if enabled.
    /// Intended to be used with an Accumulator.
    pub fn from_content_bytes(input: &'de [u8]) -> Result<Self> {
        let mut deserializer = Deserializer { input };
        // Strip instruction code from input
        let opcode = u8::deserialize(&mut deserializer)?;
        use Response::*;
        // Is it ACK?
        if opcode % 2 == 0 {
            let response = match opcode {
                Register::<Write>::OPCODE => RegisterAck,
                Word::<Write>::OPCODE => WordAck,
                Dword::<Write>::OPCODE => DwordAck,
                Curve::OPCODE => CurveAck,
                _ => return Err(ResponseUnknownInstr),
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
                Register::<Read>::OPCODE => RegisterData {
                    instr: Register::deserialize(&mut deserializer)?,
                    data: ResponseData { deserializer },
                },
                Word::<Read>::OPCODE => WordData {
                    instr: Word::deserialize(&mut deserializer)?,
                    data: ResponseData { deserializer },
                },
                Dword::<Read>::OPCODE => DwordData {
                    instr: Dword::deserialize(&mut deserializer)?,
                    data: ResponseData { deserializer },
                },
                _ => return Err(ResponseUnknownInstr),
            };
            Ok(response)
        }
    }

    /// Extracts the instruction+data part of the response from a byte slice.
    /// The unused portion (if any) of the byte slice is returned for further usage.
    /// The byte slice is expected to contain full DGUS response, including header, length, and CRC if enabled.
    fn extract_data_bytes(input: &'de [u8], crc: bool) -> Result<(&'de [u8], &'de [u8])> {
        // Strip header from input
        let input = input
            .strip_prefix(&u16::to_be_bytes(HEADER))
            .ok_or(ResponseBadHeader)?;

        // Strip length from input
        let (len, input) = input.split_first().ok_or(DeserializeUnexpectedEnd)?;
        let len = *len as usize;
        let min_len = if crc { 5 } else { 3 };
        if len < min_len || len > input.len() {
            return Err(ResponseBadLen);
        }

        // Split input with the length
        let (input, rest) = input
            .split_at_checked(len)
            .ok_or(DeserializeUnexpectedEnd)?;

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
