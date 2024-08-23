use core::fmt::{Display, Formatter};

/// Error type used by serde_dgus
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// This is a feature that serde_dgus will never implement
    WontImplement,
    /// This is a feature that serde_dgus intends to support, but does not yet
    NotYetImplemented,
    /// The serialize buffer is full
    SerializeBufferFull,
    /// Hit the end of buffer, expected more data
    DeserializeUnexpectedEnd,
    /// Found a bool that wasn't 0 or 1
    DeserializeBadBool,
    /// Bad header found during deserialization
    DeserializeBadHeader,
    /// Bad command found during deserialization
    DeserializeBadCommand,
    /// Bad CRC while deserializing
    DeserializeBadCrc,
    /// Bad header found during accumulation
    AccumulateBadHeader,
    /// Bad length found during accumulation
    AccumulateBadLen,
    /// Bad CRC found during accumulation
    AccumulateBadCrc,
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
                WontImplement => "This is a feature that serde-dgus will never implement",
                NotYetImplemented => "Serde-dgus may support this, but does not yet",
                SerializeBufferFull => "The serialize buffer is full",
                DeserializeUnexpectedEnd => "Hit the end of buffer, expected more data",
                DeserializeBadBool => "Found a bool that wasn't 0 or 1",
                DeserializeBadHeader => "Bad frame header found during deserialization",
                DeserializeBadCommand => "Bad DGUS command found during deserialization",
                DeserializeBadCrc => "Bad CRC while deserializing",
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

/// Result type used by serde-dgus.
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
