#![no_std]
#![warn(missing_docs)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ## Feature flags
#![doc = document_features::document_features!(feature_label = r#"<span class="stab portability"><code>{feature}</code></span>"#)]

mod error;
pub mod request;
pub mod response;

pub use error::{Error, Result};

use crc::{Crc, CRC_16_MODBUS};
use serde::{Deserialize, Serialize};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

trait Sealed {}

/// Instruction trait (sealed)
///
/// Implemented by all instructions. Users are responsible for valid instruction parameters such as address and length.
/// - A [`Request`][request::Request] with the instruction [`Word<Read>`] will be responded with [`WordData`][response::Response] which contains the exact instruction.
/// - A [`Request`][request::Request] with the instruction [`Register<Write>`] will responded with [`RegisterAck`][response::Response].
#[allow(private_bounds)]
pub trait Instruction: Serialize + Sealed {
    /// Instruction Code
    const CODE: u8;
}

/// Write command
///
/// Use it with an instruction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Write;
impl Sealed for Write {}

/// Read command
///
/// Use it with an instruction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Read {
    /// Length
    pub len: u8,
}
impl Sealed for Read {}

/// Register instruction
///
/// Generic over commands
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Register<C> {
    /// Register page
    pub page: u8,
    /// Register address
    pub addr: u8,
    /// Command
    pub cmd: C,
}
impl Sealed for Register<Write> {}
impl Sealed for Register<Read> {}
impl Instruction for Register<Write> {
    const CODE: u8 = 0x80;
}
impl Instruction for Register<Read> {
    const CODE: u8 = 0x81;
}

/// Word instruction
///
/// Generic over commands
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Word<C> {
    /// Address
    pub addr: u16,
    /// Command
    pub cmd: C,
}
impl Sealed for Word<Write> {}
impl Sealed for Word<Read> {}
impl Instruction for Word<Write> {
    const CODE: u8 = 0x82;
}
impl Instruction for Word<Read> {
    const CODE: u8 = 0x83;
}

/// Dword instruction
///
/// Generic over commands
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dword<C> {
    /// Address
    pub addr: u32,
    /// Command
    pub cmd: C,
}
impl Sealed for Dword<Write> {}
impl Sealed for Dword<Read> {}
impl Instruction for Dword<Write> {
    const CODE: u8 = 0x86;
}
impl Instruction for Dword<Read> {
    const CODE: u8 = 0x87;
}

/// Curve instruction
///
/// Write only
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Curve {
    /// Channel
    pub ch: u8,
}
impl Sealed for Curve {}
impl Instruction for Curve {
    const CODE: u8 = 0x84;
}
