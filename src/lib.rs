#![no_std]

use serde::{Deserialize, Serialize};

pub mod de;
pub mod dispatcher;
pub mod error;
pub mod ser;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MetaData {
    pub addr: u16,
    pub wlen: u8,
}

pub trait Variable {
    const ADDRESS: u16;

    // ? Separate this to fn address and fn wlen, get rid of metadata struct
    fn metadata() -> MetaData
    where
        Self: Sized,
    {
        const { assert!(core::mem::size_of::<Self>() % 2 == 0) }
        MetaData {
            addr: Self::ADDRESS,
            wlen: (core::mem::size_of::<Self>() / 2) as u8,
        }
    }
}

#[derive(Clone)]
pub struct Config<'a> {
    pub header: u16,
    pub crc: Option<crc::Digest<'a, u16>>,
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        use crc::{Crc, CRC_16_MODBUS};
        const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
        Self {
            header: 0x5AA5,
            crc: Some(CRC.digest()),
        }
    }
}

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

// Device functionality

// ? Let user define their own ACK type or handle it in dispatcher macro
#[derive(Deserialize)]
pub struct Ack;

impl Variable for Ack {
    const ADDRESS: u16 = u16::from_be_bytes([b'O', b'K']);
}

#[derive(Serialize, Deserialize)]
pub struct Page {
    precmd: u16,
    pub page: u16,
}

impl Variable for Page {
    const ADDRESS: u16 = 0x0084;
}

impl Page {
    fn new(id: u16) -> Self {
        Self {
            precmd: 0x5A01,
            page: id,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Background {
    precmd: u16,
    pub bg: u16,
}

impl Variable for Background {
    const ADDRESS: u16 = 0x00DE;
}

impl Background {
    fn new(id: u16) -> Self {
        Self {
            precmd: 0x5A00,
            bg: id,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Brightness {
    pub brightness: u16,
    pub timeout: u16,
}

impl Variable for Brightness {
    const ADDRESS: u16 = 0x0082;
}

impl Brightness {
    fn new(active: u8, sleep: u8, timeout: u16) -> Self {
        // ? Don't panic, set it to 100 if above 100
        assert!(active <= 100 || sleep <= 100);
        Self {
            brightness: u16::from_be_bytes([active, sleep]),
            timeout,
        }
    }
}
