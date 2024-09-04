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

/// DGUS Frame Instruction
/// 
/// Refer to T5L DGUS2 DevGuide Section 4.2
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Instruction {
    /// Write data to register
    WriteReg{page: u8, addr: u8},
    /// Read data from register
    ReadReg{page: u8, addr: u8, len: u8},
    /// Write word data to variable space with word address
    WriteWord{addr: u16},
    /// Read word data from variable space
    ReadWord{addr: u16, len: u8},
    /// Write curve buffer data
    WriteCurve{ch: u8},
    /// Write double word data to variable space, double word address
    WriteDword{addr: u32},
    /// Read word data from variable space
    ReadDword{addr: u32, len: u8},
}

impl Instruction {
    /// Get the instruction code
    pub fn code(&self) -> u8 {
        match self {
            Self::WriteReg{..} => 0x80,
            Self::ReadReg{..} => 0x81,
            Self::WriteWord{..} => 0x82,
            Self::ReadWord{..} => 0x83,
            Self::WriteCurve{..} => 0x84,
            Self::WriteDword{..} => 0x86,
            Self::ReadDword{..} => 0x87,
        }
    }
}
