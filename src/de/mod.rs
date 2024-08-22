pub(crate) mod deserializer;

use serde::Deserialize;

use crate::{
    de::deserializer::Deserializer,
    error::{Error, Result},
    Command, CRC,
};

pub struct Frame<'de> {
    pub cmd: Command,
    pub addr: u16,
    pub wlen: Option<u8>,
    pub deserializer: Deserializer<'de>,
}

impl<'de> Frame<'de> {
    /// Deserialize a message of type `T` from a data byte slice.
    /// The unused portion (if any) of the byte slice is not returned.
    pub fn from_bytes(input: &'de [u8], crc: bool) -> Result<Self> {
        let (frame, _) = Self::take_from_bytes(input, crc)?;
        Ok(frame)
    }

    pub fn take_from_bytes(input: &'de [u8], crc: bool) -> Result<(Self, &'de [u8])> {
        // Strip header from input
        let input = input
            .strip_prefix(&u16::to_be_bytes(0x5AA5))
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
            if u16::from_le_bytes(*crc) != CRC.checksum(&input[3..]) {
                return Err(Error::DeserializeBadCrc);
            }
            input
        } else {
            input
        };

        // Strip command from input
        let (cmd, input) = input.split_first().unwrap();
        let cmd = Command::from(*cmd);
        if cmd == Command::Undefined {
            return Err(Error::DeserializeBadCommand);
        }

        // Strip address from input
        let (addr, input) = input.split_first_chunk().unwrap();
        let addr = u16::from_be_bytes(*addr);

        // Strip word length from input, if there is none (could be ACK), set to 0
        let (wlen, input) = input.split_first().unwrap_or((&0, input));
        let wlen = if *wlen == 0 { None } else { Some(*wlen) };

        // Construct the frame
        Ok((
            Self {
                cmd,
                addr,
                wlen,
                deserializer: Deserializer{input},
            },
            rest,
        ))
    }

    pub fn deserialize<T: Deserialize<'de>>(&mut self) -> Result<T> {
        T::deserialize(&mut self.deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
