pub(crate) mod deserializer;

use crate::{
    de::deserializer::Deserializer,
    error::{Error, Result},
    Command, CRC, HEADER,
};
use serde::Deserialize;

pub struct RxFrame<'de> {
    pub cmd: Command,
    pub addr: u16,
    pub wlen: u8,
    pub deserializer: Deserializer<'de>,
}

impl<'de> RxFrame<'de> {
    /// Deserialize a message of type `T` from a data byte slice.
    /// The unused portion (if any) of the byte slice is not returned.
    pub fn from_bytes(input: &'de [u8], crc: bool) -> Result<Self> {
        let (frame, _) = Self::take_from_bytes(input, crc)?;
        Ok(frame)
    }

    pub fn take_from_bytes(input: &'de [u8], crc: bool) -> Result<(Self, &'de [u8])> {
        let (input, rest) = Self::extract_frame_bytes(input, crc)?;
        Ok((Self::from_data_bytes(input)?, rest))
    }

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

    fn extract_frame_bytes(input: &'de [u8], crc: bool) -> Result<(&'de [u8], &'de [u8])> {
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

    pub fn deserialize<T: Deserialize<'de>>(&mut self) -> Result<T> {
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

        let input = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];
        let expected = (Command::WriteVp, u16::from_be_bytes([b'O', b'K']), 0);
        let mut frame = RxFrame::from_bytes(&input, true).unwrap();
        let ack: Ack = frame.deserialize().unwrap();
        assert_eq!((frame.cmd, frame.addr, frame.wlen), expected);
        assert_eq!(ack, Ack);
    }
}
