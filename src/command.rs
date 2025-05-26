//! Command types

use serde::{Deserialize, Serialize};

trait Sealed {}

/// Command trait (sealed)
///
/// Implemented by all commands. Users are responsible for valid command parameters such as address and length.
/// - A [`Request`][crate::request::Request] with the command [`Word<Read>`] will be responded with [`WordData`][crate::response::Response] which contains the exact command.
/// - A [`Request`][crate::request::Request] with the command [`Register<Write>`] will responded with [`RegisterAck`][crate::response::Response].
#[allow(private_bounds)]
pub trait Command: Serialize + Sealed {
    /// Command Code
    const CMD: u8;
}

/// Write inner command
///
/// Use it with a command
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Write;
impl Sealed for Write {}

/// Read inner command
///
/// Use it with a command
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Read {
    /// Word Length
    pub wlen: u8,
}
impl Sealed for Read {}

/// Register command
///
/// Generic over inner commands
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
impl Command for Register<Write> {
    const CMD: u8 = 0x80;
}
impl Command for Register<Read> {
    const CMD: u8 = 0x81;
}

/// Word command
///
/// Generic over inner commands
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
impl Command for Word<Write> {
    const CMD: u8 = 0x82;
}
impl Command for Word<Read> {
    const CMD: u8 = 0x83;
}

/// Dword command
///
/// Generic over inner commands
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
impl Command for Dword<Write> {
    const CMD: u8 = 0x86;
}
impl Command for Dword<Read> {
    const CMD: u8 = 0x87;
}

/// Curve command
///
/// Write only
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Curve {
    /// Channel
    pub ch: u8,
}
impl Sealed for Curve {}
impl Command for Curve {
    const CMD: u8 = 0x84;
}
