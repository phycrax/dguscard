#![no_std]
#![warn(missing_docs)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ## Feature flags
#![doc = document_features::document_features!(feature_label = r#"<span class="stab portability"><code>{feature}</code></span>"#)]

mod error;

pub mod command;
pub mod request;
pub mod response;

use crc::{Crc, CRC_16_MODBUS};
pub use error::{Error, Result};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

/// Frame address
///
/// Users are responsible for valid address parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Address {
    /// Register address space (write `0x80`, read `0x81`)
    Register(u8, u8),
    /// Variable address space (write `0x82`, read `0x83`)
    Word(u16),
    /// Extended variable address space (write `0x86`, read `0x87`)
    Dword(u32),
    /// Curve buffer (write only `0x84`)
    Curve(u8),
}

impl Address {
    const REGISTER: u8 = 0x80;
    const WORD: u8 = 0x82;
    const CURVE: u8 = 0x84;
    const DWORD: u8 = 0x86;
}
