#![no_std]
#![warn(missing_docs)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ## Feature flags
#![doc = document_features::document_features!(feature_label = r#"<span class="stab portability"><code>{feature}</code></span>"#)]

mod error;
pub mod response;
pub mod request;

pub use error::{Error, Result};
pub use request::Frame as RequestFrame;
pub use response::Frame as ResponseFrame;

use crc::{Crc, CRC_16_MODBUS};
const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

/// DGUS Frame Instruction
///
/// Refer to T5L_DGUS2 DevGuide Section 4.2
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(missing_docs)]
pub enum Instruction {
    /// Write data to register
    WriteReg { page: u8, addr: u8 },
    /// Read data from register
    ReadReg { page: u8, addr: u8, len: u8 },
    /// Write word data to variable space with word address
    WriteWord { addr: u16 },
    /// Read word data from variable space
    ReadWord { addr: u16, len: u8 },
    /// Write curve buffer data
    WriteCurve { ch: u8 },
    /// Write double word data to variable space, double word address
    WriteDword { addr: u32 },
    /// Read word data from variable space
    ReadDword { addr: u32, len: u8 },
}
