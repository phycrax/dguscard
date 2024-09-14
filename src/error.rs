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
    /// TODO docs
    SerializeVariantIndexTooLarge,
    /// Hit the end of buffer, expected more data
    DeserializeUnexpectedEnd,
    /// Found a bool that wasn't 0 or 1
    DeserializeBadBool,
    /// Bad header found during deserialization
    DeserializeBadOption,
    /// TODO docs
    DeserializeBadHeader,
    /// Bad command found during deserialization
    DeserializeBadCommand,
    /// TODO docs
    DeserializeBadInstruction,
    /// Bad CRC while deserializing
    DeserializeBadCrc,
    /// Bad header found during accumulation
    AccumulateBadHeader,
    /// Bad length found during accumulation
    AccumulateBadLen,
    /// Bad CRC found during accumulation
    AccumulateBadCrc,
    /// TODO docs
    AccumulateBufferFull,
    /// Serde Serialization Error
    SerdeSerCustom,
    /// Serde Deserialization Error
    SerdeDeCustom,
}

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
                SerializeVariantIndexTooLarge => "The serialize buffer is full",
                DeserializeUnexpectedEnd => "Hit the end of buffer, expected more data",
                DeserializeBadBool => "Found a bool that wasn't 0 or 1",
                DeserializeBadOption => "Found a bool that wasn't 0 or 1",
                DeserializeBadHeader => "Bad frame header found during deserialization",
                DeserializeBadCommand => "Bad DGUS command found during deserialization",
                DeserializeBadCrc => "Bad CRC while deserializing",
                DeserializeBadInstruction => "Bad CRC while deserializing",
                AccumulateBadHeader => "Bad frame header found during accumulation",
                AccumulateBadLen => "Bad frame header found during accumulation",
                AccumulateBadCrc => "Bad frame header found during accumulation",
                AccumulateBufferFull => "Bad frame header found during accumulation",
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

impl serde::ser::StdError for Error {}
