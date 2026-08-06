#![no_std]
#![warn(missing_docs)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ## Feature flags
#![doc = document_features::document_features!(feature_label = r#"<span class="stab portability"><code>{feature}</code></span>"#)]

pub mod request;
pub mod response;

const CRC: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_MODBUS);
const HEADER: u16 = 0x5AA5;

const FRAME_OVERHEAD: usize = 3;
const MAX_CONTENT_LEN: usize = u8::MAX as usize;
const MAX_FRAME_LEN: usize = FRAME_OVERHEAD + MAX_CONTENT_LEN;

const CRC_LEN: usize = 2;
const MIN_CONTENT_LEN: usize = 3;

trait Sealed {}

/// Computes the CRC-16/MODBUS checksum DGUS frames carry.
///
/// Implement it to drive an MCU CRC peripheral instead of the software impl.
pub trait Checksum {
    /// Computes the checksum over a frame's content.
    fn compute(&mut self, data: &[u8]) -> u16;
}

/// CRC-16/MODBUS software impl.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Crc;

impl Checksum for Crc {
    fn compute(&mut self, data: &[u8]) -> u16 {
        CRC.checksum(data)
    }
}

/// Lets a borrowed peripheral be used wherever an owned one can.
impl<T: Checksum + ?Sized> Checksum for &mut T {
    fn compute(&mut self, data: &[u8]) -> u16 {
        (**self).compute(data)
    }
}

impl Crc {
    /// Frames carry a CRC-16/MODBUS trailer.
    pub const fn on() -> Option<Self> {
        Some(Self)
    }

    /// Frames carry no checksum.
    pub const fn off() -> Option<Self> {
        None
    }
}

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

    const fn code(self, is_read: bool) -> u8 {
        let code = match self {
            Address::Register(..) => Self::REGISTER,
            Address::Word(..) => Self::WORD,
            Address::Dword(..) => Self::DWORD,
            Address::Curve(..) => Self::CURVE,
        };

        code + is_read as u8
    }
}
