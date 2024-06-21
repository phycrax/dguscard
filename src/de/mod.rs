pub(crate) mod deserializer;

use crate::{
    de::deserializer::Deserializer,
    error::{Error, Result},
    Command, Config, DwinVariable, MetaData,
};
use serde::Deserialize;

pub struct RawBytes<'a>(&'a [u8]);

pub fn from_bytes<'a, T>(input: &'a [u8], cfg: Config) -> Result<T>
where
    T: DwinVariable + Deserialize<'a>,
{
    let (input_md, rawdata) = metadata_from_bytes(input, cfg)?;
    let type_md = T::metadata();

    if type_md.addr != input_md.addr {
        return Err(Error::DeserializeUnexpectedAddr);
    }
    if type_md.wlen != input_md.wlen {
        return Err(Error::DeserializeUnexpectedWlen);
    }
    from_raw_bytes(rawdata)
}

pub fn metadata_from_bytes<'a>(input: &'a [u8], cfg: Config) -> Result<(MetaData, RawBytes<'a>)> {
    // Slice too short?
    let min_len = if cfg.crc.is_some() { 8 } else { 6 };
    if input.len() < min_len {
        return Err(Error::DeserializeBadBufferLen);
    }

    // Strip header from input
    let input = input
        .strip_prefix(&u16::to_be_bytes(cfg.header))
        .ok_or(Error::DeserializeBadHeader)?;

    // Strip length from input
    let (length, input) = input.split_first().unwrap();
    let length = *length as usize;

    // Trim slice with the length
    let input = input.get(..length).ok_or(Error::DeserializeBadBufferLen)?;

    // Strip CRC from input
    let input = if let Some(mut digest) = cfg.crc {
        let (input, crc) = input.split_last_chunk().unwrap();
        digest.update(input);
        if u16::from_le_bytes(*crc) != digest.finalize() {
            return Err(Error::DeserializeBadCrc);
        }
        input
    } else {
        input
    };

    // Strip command from input
    let (command, input) = input.split_first().unwrap();
    let command = Command::from(*command);
    if command == Command::Undefined {
        return Err(Error::DeserializeBadCmd);
    }

    // Strip address from input
    let (addr, input) = input.split_first_chunk().unwrap();
    let addr = u16::from_be_bytes(*addr);

    // Strip word length from input, if there is none (could be ACK), set to 0
    let (wlen, input) = input.split_first().unwrap_or((&0, input));
    let wlen = *wlen;

    Ok((MetaData { addr, wlen }, RawBytes(input)))
}

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
        let expected = MetaData {
            addr: u16::from_be_bytes([b'O', b'K']),
            wlen: 0,
        };
        let (md, _) = metadata_from_bytes(&input, Default::default()).unwrap();
        assert_eq!(md, expected);
    }

    #[test]
    fn md_bad_hdr() {
        let input = [0xAA, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];
        let Err(Error::DeserializeBadHeader) = metadata_from_bytes(&input, Default::default())
        else {
            panic!();
        };
    }

    #[test]
    fn md_bad_crc() {
        let input = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xAA, 0xEF, 0, 0, 0, 0];
        let Err(Error::DeserializeBadCrc) = metadata_from_bytes(&input, Default::default()) else {
            panic!();
        };
    }

    #[test]
    fn md_bad_cmd() {
        let input = [0x5A, 0xA5, 5, 0xAA, b'O', b'K', 0x25, 0xE7, 0, 0, 0, 0];
        let Err(Error::DeserializeBadCmd) = metadata_from_bytes(&input, Default::default()) else {
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

        impl DwinVariable for Expected {
            const ADDRESS: u16 = 0xAABB;
        }

        let expected = Expected { data: 0xCCDD };

        let actual: Expected = from_bytes(&input, Default::default()).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn fb_unexp_wlen() {
        let input = [
            0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D, 0, 0, 0, 0,
        ];

        #[derive(Deserialize, Debug, PartialEq, Eq)]
        struct Expected {
            data1: u16,
            data2: u16,
        }

        impl DwinVariable for Expected {
            const ADDRESS: u16 = 0xAABB;
        }

        let result: Result<Expected> = from_bytes(&input, Default::default());
        let Err(Error::DeserializeUnexpectedWlen) = result else {
            panic!();
        };
    }
}
