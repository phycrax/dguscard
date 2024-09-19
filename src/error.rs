use core::fmt::{Display, Formatter};

/// Error type used by dguscard
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// This is a feature that dguscard will never implement
    WontImplement,
    /// This is a feature that dguscard intends to support, but does not yet
    NotYetImplemented,
    /// The serialize buffer is full
    SerializeBufferFull,
    /// Found an enum discriminant that was > u16::max_value()
    SerializeBadEnum,
    /// Hit the end of buffer, expected more data
    DeserializeUnexpectedEnd,
    /// Found a bool that wasn't 0 or 1
    DeserializeBadBool,
    /// Found an Option discriminant that wasn't 0 or 1
    DeserializeBadOption,
    /// Header mismatch found during frame deserialization
    DeserializeBadHeader,
    /// Frame length is larger than the buffer/input size or smaller than the minimum frame size
    DeserializeBadLen,
    /// Unknown frame instruction found during frame deserialization
    DeserializeBadInstruction,
    /// CRC mismatch found during frame deserialization
    DeserializeBadCrc,
    /// The accumulator buffer is full
    AccumulateBufferFull,
    /// Serde Serialization Error
    SerdeSerCustom,
    /// Serde Deserialization Error
    SerdeDeCustom,
}

impl core::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        use Error::*;
        write!(
            f,
            "{}",
            match self {
                WontImplement => "This is a feature that dguscard will never implement",
                NotYetImplemented => "dguscard may support this, but does not yet",
                SerializeBufferFull => "The serialize buffer is full",
                SerializeBadEnum => "Found an enum discriminant that was > u16::max_value()",
                DeserializeUnexpectedEnd => "Hit the end of buffer, expected more data",
                DeserializeBadBool => "Found a bool that wasn't 0 or 1",
                DeserializeBadOption => "Found an Option discriminant that wasn't 0 or 1",
                DeserializeBadHeader => "Header mismatch found during frame deserialization",
                DeserializeBadLen => "Frame length is larger than the buffer/input size or smaller than the minimum frame size",
                DeserializeBadInstruction => "Unknown frame instruction found during frame deserialization",
                DeserializeBadCrc => "CRC mismatch found during frame deserialization",
                AccumulateBufferFull => "The accumulator buffer is full",
                SerdeSerCustom => "Serde Serialization Error",
                SerdeDeCustom => "Serde Deserialization Error",
            }
        )
    }
}

/// Result type used by dguscard
pub type Result<T> = ::core::result::Result<T, Error>;

impl serde::ser::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: Display,
    {
        Error::SerdeSerCustom
    }
}

impl serde::de::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: Display,
    {
        Error::SerdeDeCustom
    }
}
