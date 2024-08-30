#![no_std]
#![warn(missing_docs)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod error;
/// Receiving data from DGUS
pub mod rx;
/// Sending data to DGUS
pub mod tx;

pub use error::{Error, Result};

use crc::{Crc, CRC_16_MODBUS};
const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

/// DGUS Frame Commands
/// 
/// Refer to T5L DGUS2 DevGuide Section 4.2
#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    /// Write data to register
    WriteReg = 0x80,
    /// Read data from register
    ReadReg,
    /// Write word data to variable space with word address
    WriteWord,
    /// Read word data from variable space
    ReadWord,
    /// Write curve buffer data
    WriteCurve,
    /// Unknown or not yet supported command
    Unknown,
    /// Write double word data to variable space, double word address
    WriteDword,
    /// Read word data from variable space
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
            0x84 => WriteCurve,
            0x86 => WriteDword,
            0x87 => ReadDword,
            _ => Unknown,
        }
    }
}
