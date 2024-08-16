#![no_std]

mod de;
mod error;
mod ser;

pub use de::deserializer::Deserializer;
// pub use de::{from_bytes, from_bytes_cobs, take_from_bytes, take_from_bytes_cobs};
pub use error::{Error, Result};
pub use ser::storage as ser_storage;
pub use ser::{serializer::Serializer, to_hvec, to_slice};

/// Metadata
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Metadata {
    pub addr: u16,
    pub wlen: u8,
}

/// DGUS Configuration for the packet serialization and deserialization
#[derive(Clone)]
pub struct Config {
    pub header: u16,
    pub crc: Option<crc::Digest<'static, u16>>,
}

impl Default for Config {
    fn default() -> Self {
        use crc::{Crc, CRC_16_MODBUS};
        const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
        Self {
            header: 0x5AA5,
            crc: Some(CRC.digest()),
        }
    }
}

/// DGUS Commands
/// Currently Read word and Write word are supported
#[repr(u8)]
#[derive(PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    Write = 0x82,
    Read,
    // ToDo other cmds
    Undefined,
}

impl From<u8> for Command {
    fn from(value: u8) -> Self {
        use Command::*;
        match value {
            0x82 => Write,
            0x83 => Read,
            _ => Undefined,
        }
    }
}
