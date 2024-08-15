pub(crate) mod deserializer;

use crate::{
    de::deserializer::Deserializer,
    error::{Error, Result},
    Command, Config, Metadata,
};
use serde::Deserialize;

pub struct RawBytes<'a>(pub &'a [u8]);

/// Splits metadata from a byte slice and returns the metadata and the remaining bytes.
pub fn split_metadata<'a>(input: &'a [u8], cfg: Config) -> Result<(Metadata, RawBytes<'a>)> {
    // Slice too short?
    let min_len = if cfg.crc.is_some() { 8 } else { 6 };
    if input.len() < min_len {
        return Err(Error::DeserializeBadBufferLen1);
    }

    // Strip header from input
    let input = input
        .strip_prefix(&u16::to_be_bytes(cfg.header))
        .ok_or(Error::DeserializeBadHeader)?;

    // Strip length from input
    let (len, input) = input.split_first().ok_or(Error::DeserializeBadBufferLen2)?;
    let len = *len as usize;

    // Trim slice with the length
    let input = input.get(..len).ok_or(Error::DeserializeBadBufferLen3)?;

    // Strip CRC from input
    let input = if let Some(mut digest) = cfg.crc {
        let (input, crc) = input
            .split_last_chunk()
            .ok_or(Error::DeserializeBadBufferLen4)?;
        digest.update(input);
        if u16::from_le_bytes(*crc) != digest.finalize() {
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
        return Err(Error::DeserializeBadCmd);
    }

    // Strip address from input
    let (addr, input) = input.split_first_chunk().unwrap();
    let addr = u16::from_be_bytes(*addr);

    // Strip word length from input, if there is none (could be ACK), set to 0
    let (wlen, input) = input.split_first().unwrap_or((&0, input));
    let wlen = *wlen;

    // Calculate the actual raw

    Ok((Metadata { addr, wlen }, RawBytes(input)))
}

/// Deserialize a message of type `T` from a data byte slice.
/// The unused portion (if any) of the byte slice is not returned.
pub fn from_raw_bytes<'a, T>(input: RawBytes<'a>) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(input.0);
    T::deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md_ack() {
        let input = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];
        let expected = Metadata {
            addr: u16::from_be_bytes([b'O', b'K']),
            wlen: 0,
        };
        let (md, _) = split_metadata(&input, Default::default()).unwrap();
        assert_eq!(md, expected);
    }

    #[test]
    fn md_bad_hdr() {
        let input = [0xAA, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];
        let Err(Error::DeserializeBadHeader) = split_metadata(&input, Default::default()) else {
            panic!();
        };
    }

    #[test]
    fn md_bad_crc() {
        let input = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xAA, 0xEF, 0, 0, 0, 0];
        let Err(Error::DeserializeBadCrc) = split_metadata(&input, Default::default()) else {
            panic!();
        };
    }

    #[test]
    fn md_bad_cmd() {
        let input = [0x5A, 0xA5, 5, 0xAA, b'O', b'K', 0x25, 0xE7, 0, 0, 0, 0];
        let Err(Error::DeserializeBadCmd) = split_metadata(&input, Default::default()) else {
            panic!();
        };
    }

    #[test]
    fn fb_u16_struct() {
        let input = [
            0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D, 0, 0, 0, 0,
        ];

        #[derive(Deserialize, Debug, PartialEq, Eq)]
        struct Expected {
            data: u16,
        }

        let (metadata, raw_bytes) = split_metadata(&input, Default::default()).unwrap();
        assert_eq!(
            metadata,
            Metadata {
                addr: 0xAABB,
                wlen: 1
            }
        );

        let expected = Expected { data: 0xCCDD };
        let actual: Expected = from_raw_bytes(raw_bytes).unwrap();
        assert_eq!(expected, actual);
    }
}
