use core::fmt::{Display, Formatter};

/// Result type used by dguscard
pub type Result<T> = ::core::result::Result<T, Error>;

/// Error type used by dguscard
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// The request buffer is full
    RequestBufferFull,
    /// Response header mismatch
    ResponseBadHeader,
    /// Response length is more than the buffer size
    ResponseTooLarge,
    /// Response length is less than the minimum proper response length
    ResponseBadLen,
    /// Unknown response instruction
    ResponseUnknownInstr,
    /// Bad Ack response
    ResponseBadAck,
    /// Response CRC mismatch
    ResponseBadCrc,
    /// Found an enum discriminant that was > u16::max_value()
    SerializeBadEnum,
    /// Hit the end of buffer, expected more data
    DeserializeUnexpectedEnd,
    /// Found a bool that wasn't 0 or 1
    DeserializeBadBool,
    /// Found an Option discriminant that wasn't 0 or 1
    DeserializeBadOption,
    /// The accumulator buffer is full
    AccumulateBufferFull,
    /// dguscard will never implement this
    WontImplement,
    /// dguscard may support this
    NotYetImplemented,
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
                RequestBufferFull => "The request buffer is full",
                ResponseBadHeader => "Response header mismatch",
                ResponseTooLarge => "Response length is more than the buffer size",
                ResponseBadLen => "Response length is less than the minimum proper response length",
                ResponseUnknownInstr => "Unknown response instruction",
                ResponseBadAck => "Bad Ack response",
                ResponseBadCrc => "Response CRC mismatch",
                SerializeBadEnum => "Found an enum discriminant that was > u16::max_value()",
                DeserializeUnexpectedEnd => "Hit the end of buffer, expected more data",
                DeserializeBadBool => "Found a bool that wasn't 0 or 1",
                DeserializeBadOption => "Found an Option discriminant that wasn't 0 or 1",
                AccumulateBufferFull => "The accumulator buffer is full",
                WontImplement => "dguscard will never implement this",
                NotYetImplemented => "dguscard may support this",
                SerdeSerCustom => "Serde Serialization Error",
                SerdeDeCustom => "Serde Deserialization Error",
            }
        )
    }
}

impl core::error::Error for Error {}

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
