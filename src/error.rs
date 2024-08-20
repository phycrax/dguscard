use core::fmt::{Display, Formatter};

/// Error type used by serde_dgus
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
//#[non_exhaustive]
pub enum Error {
    /// This is a feature that serde_dgus will never implement
    WontImplement,
    /// This is a feature that serde_dgus intends to support, but does not yet
    NotYetImplemented,
    /// The serialize buffer is full
    SerializeBufferFull,
    /// Bad buffer length
    DeserializeBadBufferLen1,
    DeserializeBadBufferLen2,
    DeserializeBadBufferLen3,
    DeserializeBadBufferLen4,
    /// Bad header
    DeserializeBadHeader,
    /// Bad command
    DeserializeBadCmd,
    /// Unexpected address
    DeserializeUnexpectedAddr,
    /// Unexpected word length
    DeserializeUnexpectedWlen,
    /// Hit the end of buffer, expected more data
    DeserializeUnexpectedEnd,
    /// Found a varint that didn't terminate. Is the usize too big for this platform?
    DeserializeBadVarint,
    /// Found a bool that wasn't 0 or 1
    DeserializeBadBool,
    /// Found an invalid unicode char
    DeserializeBadChar,
    /// Tried to parse invalid utf-8
    DeserializeBadUtf8,
    /// Found an Option discriminant that wasn't 0 or 1
    DeserializeBadOption,
    /// Found an enum discriminant that was > u32::max_value()
    DeserializeBadEnum,
    /// The original data was not well encoded
    DeserializeBadEncoding,
    /// Bad CRC while deserializing
    DeserializeBadCrc,
    /// Serde Serialization Error
    SerdeSerCustom,
    /// Serde Deserialization Error
    SerdeDeCustom,
    /// Error while processing `collect_str` during serialization
    CollectStrError,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        use Error::*;
        write!(
            f,
            "{}",
            match self {
                WontImplement => "This is a feature that serde-dgus will never implement",
                NotYetImplemented => {
                    "This is a feature that serde-dgus intends to support, but does not yet"
                }
                SerdeSerCustom => "Serde Serialization Error",
                SerdeDeCustom => "Serde Deserialization Error",
                SerializeBufferFull => "The serialize buffer is full",                DeserializeUnexpectedEnd => "Hit the end of buffer, expected more data",
                DeserializeUnexpectedAddr =>
                    "Error while processing `collect_str` during serialization",
                DeserializeUnexpectedWlen =>
                    "Error while processing `collect_str` during serialization",
                DeserializeBadVarint => {
                    "Found a varint that didn't terminate. Is the usize too big for this platform?"
                }
                DeserializeBadBool => "Found a bool that wasn't 0 or 1",
                DeserializeBadChar => "Found an invalid unicode char",
                DeserializeBadUtf8 => "Tried to parse invalid utf-8",
                DeserializeBadOption => "Found an Option discriminant that wasn't 0 or 1",
                DeserializeBadEnum => "Found an enum discriminant that was > u32::max_value()",
                DeserializeBadEncoding => "The original data was not well encoded",
                DeserializeBadCrc => "Bad CRC while deserializing",
                CollectStrError => "Error while processing `collect_str` during serialization",
                DeserializeBadBufferLen1 =>
                    "Error while processing `collect_str` during serialization",
                DeserializeBadBufferLen2 =>
                    "Error while processing `collect_str` during serialization",
                DeserializeBadBufferLen3 =>
                    "Error while processing `collect_str` during serialization",
                DeserializeBadBufferLen4 =>
                    "Error while processing `collect_str` during serialization",
                DeserializeBadHeader => "Error while processing `collect_str` during serialization",
                DeserializeBadCmd => "Error while processing `collect_str` during serialization",
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
