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
    WriteVp,
    ReadVp,
    // ToDo other cmds
    Undefined,
}

impl From<u8> for Command {
    fn from(value: u8) -> Self {
        use Command::*;
        match value {
            0x82 => WriteVp,
            0x83 => ReadVp,
            _ => Undefined,
        }
    }
}
