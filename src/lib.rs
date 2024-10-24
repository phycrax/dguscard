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

const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

trait Instruction: Serialize {
    const OPCODE: u8;
}

/// Write command
///
/// Use it with an instruction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Write;

/// Read command
///
/// Use it with an instruction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Read {
    /// Length
    pub len: u8,
}

/// Register instruction
///
/// Generic over commands
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Register<T> {
    /// Register page
    pub page: u8,
    /// Register address
    pub addr: u8,
    /// Command
    pub cmd: T,
}
impl Instruction for Register<Write> {
    const OPCODE: u8 = 0x80;
}
impl Instruction for Register<Read> {
    const OPCODE: u8 = 0x81;
}

/// Word instruction
///
/// Generic over commands
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Word<T> {
    /// Address
    pub addr: u16,
    /// Command
    pub cmd: T,
}
impl Instruction for Word<Write> {
    const OPCODE: u8 = 0x82;
}
impl Instruction for Word<Read> {
    const OPCODE: u8 = 0x83;
}

/// Dword instruction
///
/// Generic over commands
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dword<T> {
    /// Address
    pub addr: u32,
    /// Command
    pub cmd: T,
}
impl Instruction for Dword<Write> {
    const OPCODE: u8 = 0x86;
}
impl Instruction for Dword<Read> {
    const OPCODE: u8 = 0x87;
}

/// Curve instruction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Curve {
    /// Channel
    pub ch: u8,
}
impl Instruction for Curve {
    const OPCODE: u8 = 0x84;
}
