#![no_std]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod error;
pub mod rx;
pub mod tx;

pub use error::{Error, Result};

use crc::{Crc, CRC_16_MODBUS};
const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

/// DGUS Commands
#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    WriteReg = 0x80,
    ReadReg,
    WriteWord,
    ReadWord,
    Curve,
    Unknown,
    WriteDword,
    ReadDword,
}

impl From<u8> for Command {
    fn from(value: u8) -> Self {
        use Command::*;
        match value {
            0x80 => WriteReg,
            0x81 => ReadReg,
            0x82 => WriteWord,
            0x83 => ReadWord,
            0x84 => Curve,
            0x86 => WriteDword,
            0x87 => ReadDword,
            _ => Unknown,
        }
    }
}
