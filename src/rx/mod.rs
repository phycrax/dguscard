mod accumulator;
mod deserializer;

pub use accumulator::{Accumulator, FeedResult};

use crate::{
    error::{Error, Result},
    rx::deserializer::Deserializer,
    Command, CRC, HEADER,
};
use serde::Deserialize;

/// A RX frame with the capability of deserializing values from itself.
pub struct RxFrame<'de> {
    /// Command of the received frame
    pub cmd: Command,
    /// Address of the received frame
    pub addr: u16,
    /// Word length of the received frame
    pub wlen: u8,
    /// Deserializer for the data section of the frame
    deserializer: Deserializer<'de>,
}

impl<'de> RxFrame<'de> {
    /// Try to grab a frame from a byte slice.
    /// The byte slice is expected to contain full DGUS frame, including header, length, and CRC.
    /// The unused portion (if any) of the byte slice is not returned.
    pub fn from_bytes(input: &'de [u8], crc: bool) -> Result<Self> {
        let (frame, _) = Self::take_from_bytes(input, crc)?;
        Ok(frame)
    }

    /// Try to take a frame from a byte slice.
    /// The byte slice is expected to contain full DGUS frame, including header, length, and optional CRC.
    /// The unused portion (if any) of the byte slice is returned for further usage.
    pub fn take_from_bytes(input: &'de [u8], crc: bool) -> Result<(Self, &'de [u8])> {
        let (input, rest) = Self::extract_data_bytes(input, crc)?;
        Ok((Self::from_data_bytes(input)?, rest))
    }

    /// Try to grab a headless and tailless frame from a byte slice.
    /// The byte slice is expected to contain headless and tailless DGUS frame, i.e. excluding header, length, and optional CRC.
    /// The unused portion (if any) of the byte slice is not returned.
    /// Intended to be used with an Accumulator.
    pub fn from_data_bytes(input: &'de [u8]) -> Result<Self> {
        // Strip command from input
        let (&cmd, input) = input.split_first().unwrap();
        let cmd = Command::from(cmd);
        if cmd == Command::Undefined {
            return Err(Error::DeserializeBadCommand);
        }

        // Strip address from input
        let (&addr, input) = input.split_first_chunk().unwrap();
        let addr = u16::from_be_bytes(addr);

        // Strip word length from input, if there is none (could be ACK), set to 0
        let (&wlen, input) = input.split_first().unwrap_or((&0, input));

        // Construct the frame
        Ok(Self {
            cmd,
            addr,
            wlen,
            deserializer: Deserializer { input },
        })
    }

    /// Extracts the data part of the frame from a byte slice.
    /// The byte slice is expected to contain full DGUS frame, including header, length, and optional CRC.
    fn extract_data_bytes(input: &'de [u8], crc: bool) -> Result<(&'de [u8], &'de [u8])> {
        // Strip header from input
        let input = input
            .strip_prefix(&u16::to_be_bytes(HEADER))
            .ok_or(Error::DeserializeBadHeader)?;

        // Strip length from input
        let (len, input) = input.split_first().ok_or(Error::DeserializeUnexpectedEnd)?;
        let len = *len as usize;

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

    /// Splits and deserializes a value from the frame.
    pub fn split_value<T: Deserialize<'de>>(&mut self) -> Result<T> {
        T::deserialize(&mut self.deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Ack;

        let input = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 1, 2, 3, 4];
        let expected = (Command::WriteVp, u16::from_be_bytes([b'O', b'K']), 0);
        let (mut frame, rest) = RxFrame::take_from_bytes(&input, true).unwrap();
        let ack: Ack = frame.split_value().unwrap();
        assert_eq!((frame.cmd, frame.addr, frame.wlen), expected);
        assert_eq!(ack, Ack);
        assert_eq!(rest, &[1, 2, 3, 4]);
    }
}
