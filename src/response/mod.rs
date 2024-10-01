//! Response frame parser/deserializer/accumulator

mod accumulator;
mod deserializer;

pub use accumulator::{Accumulator, FeedResult};

use crate::{
    error::{Error, Result},
    response::deserializer::Deserializer,
    Instruction, CRC, HEADER,
};
use serde::Deserialize;

/// Response frame parser
/// 
/// # Examples
/// 
/// ```rust
/// use dguscard::{ResponseFrame, Instruction};
/// use std::io::Read;
/// #[derive(serde::Deserialize)]
/// struct MyData {
///     a: u8,
///     b: u16,
///     c: u32,
/// }
/// let mut uart = /* Anything that implements the `Read` trait */
/// # std::collections::VecDeque::from([
/// # 0x5A, 0xA5, 14, 0x82, 0x12, 0x34, 0x11, 0x22, 0x22, 
/// # 0x33, 0x33, 0x33, 0x33, 0x44, 0x44, 0x44, 0x44]);
/// // Backing buffer for the UART.
/// let buf = &mut [0u8; 50];
/// // Read a frame from UART.
/// let _ = uart.read(buf).unwrap();
/// // Look for a frame within the buffer.
/// let mut frame = ResponseFrame::from_bytes(buf, false).unwrap();
/// // Do something with the received instruction.
/// dbg!(frame.instr);
/// // Take a MyData from the frame
/// let data: MyData = frame.take().unwrap();
/// // Take an u32 from the frame
/// let integer: u32 = frame.take().unwrap();
/// ```
/// 
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Frame<'de> {
    /// Instruction of the received response
    pub instr: Instruction,
    /// Deserializer for the data section of the frame
    deserializer: Deserializer<'de>,
}

impl<'de> Frame<'de> {
    /// Looks for a frame within a byte slice.
    /// The unused portion (if any) of the byte slice is not returned.
    /// The byte slice is expected to contain full DGUS frame, including header, length, and CRC if enabled.
    pub fn from_bytes(input: &'de [u8], crc: bool) -> Result<Self> {
        let (frame, _) = Self::take_from_bytes(input, crc)?;
        Ok(frame)
    }

    /// Looks for a frame within a byte slice.
    /// The unused portion (if any) of the byte slice is returned for further usage.
    /// The byte slice is expected to contain full DGUS frame, including header, length, and CRC if enabled.
    pub fn take_from_bytes(input: &'de [u8], crc: bool) -> Result<(Self, &'de [u8])> {
        let (input, rest) = Self::extract_data_bytes(input, crc)?;
        Ok((Self::from_data_bytes(input)?, rest))
    }

    /// Looks for a frame within a byte slice.
    /// The unused portion (if any) of the byte slice is not returned.
    /// The data byte slice is expected to contain instruction and data part of the DGUS frame,
    /// i.e. excluding header, length, and CRC if enabled.
    /// Intended to be used with an Accumulator.
    pub fn from_data_bytes(input: &'de [u8]) -> Result<Self> {
        // Strip instruction code from input
        let (&instr_code, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;

        // Strip instruction details from input and create the instruction
        let (instr, input) = match instr_code {
            0x80 => {
                let (&page, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
                let (&addr, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
                (Instruction::WriteReg { page, addr }, input)
            }
            0x81 => {
                let (&page, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
                let (&addr, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
                let (&len, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
                (Instruction::ReadReg { page, addr, len }, input)
            }
            0x82 => {
                let (&addr, input) = input.split_first_chunk().ok_or(Error::DeserializeUnexpectedEnd)?;
                (
                    Instruction::WriteWord {
                        addr: u16::from_be_bytes(addr),
                    },
                    input,
                )
            }
            0x83 => {
                let (&addr, input) = input.split_first_chunk().ok_or(Error::DeserializeUnexpectedEnd)?;
                let (&len, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
                (
                    Instruction::ReadWord {
                        addr: u16::from_be_bytes(addr),
                        len,
                    },
                    input,
                )
            }
            0x84 => {
                let (&ch, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
                (Instruction::WriteCurve { ch }, input)
            }
            0x86 => {
                let (&addr, input) = input.split_first_chunk().ok_or(Error::DeserializeUnexpectedEnd)?;
                (
                    Instruction::WriteDword {
                        addr: u32::from_be_bytes(addr),
                    },
                    input,
                )
            }
            0x87 => {
                let (&addr, input) = input.split_first_chunk().ok_or(Error::DeserializeUnexpectedEnd)?;
                let (&len, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
                (
                    Instruction::ReadDword {
                        addr: u32::from_be_bytes(addr),
                        len,
                    },
                    input,
                )
            }
            _ => return Err(Error::DeserializeBadInstruction),
        };

        // Return the frame
        Ok(Self {
            instr,
            deserializer: Deserializer { input },
        })
    }

    /// Extracts the instruction+data part of the frame from a byte slice.
    /// The unused portion (if any) of the byte slice is returned for further usage.
    /// The byte slice is expected to contain full DGUS frame, including header, length, and CRC if enabled.
    fn extract_data_bytes(input: &'de [u8], crc: bool) -> Result<(&'de [u8], &'de [u8])> {
        // Strip header from input
        let input = input
            .strip_prefix(&u16::to_be_bytes(HEADER))
            .ok_or(Error::DeserializeBadHeader)?;

        // Strip length from input
        let (len, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
        let len = *len as usize;
        let min_len = if crc { 5 } else { 3 };
        if len < min_len || len > input.len() {
            return Err(Error::DeserializeBadLen);
        }

        // Split input with the length
        let (input, rest) = input
            .split_at_checked(len)
            .ok_or(Error::DeserializeUnexpectedEnd)?;

        // Strip CRC from input
        let input = if crc {
            let (input, crc) = input
                .split_last_chunk()
                .ok_or(Error::DeserializeUnexpectedEnd)?;
            if u16::from_le_bytes(*crc) != CRC.checksum(input) {
                return Err(Error::DeserializeBadCrc);
            }
            input
        } else {
            input
        };

        Ok((input, rest))
    }

    /// Removes a `T` from the frame and returns it.
    pub fn take<T: Deserialize<'de>>(&mut self) -> Result<T> {
        T::deserialize(&mut self.deserializer)
    }

    /// Returns the number of remaining bytes in the frame.
    pub fn len(&self) -> usize {
        self.deserializer.input.len()
    }

    /// Returns true if the frame does not contain any remaining bytes.
    pub fn is_empty(&self) -> bool {
        self.deserializer.input.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_crc() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Ack;

        let input = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 1, 2, 3, 4];
        let expected = Instruction::WriteWord {
            addr: u16::from_be_bytes([b'O', b'K']),
        };
        let (mut frame, rest) = Frame::take_from_bytes(&input, true).unwrap();
        let ack: Ack = frame.take().unwrap();
        assert_eq!(frame.instr, expected);
        assert_eq!(ack, Ack);
        assert_eq!(rest, &[1, 2, 3, 4]);
    }
}
