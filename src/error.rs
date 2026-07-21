use core::fmt::Formatter;

/// Result type used by dguscard
pub type Result<T> = ::core::result::Result<T, Error>;

/// Error type used by dguscard
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Request buffer is full
    RequestBufferFull,
    /// Bad response header
    ResponseBadHeader,
    /// Response exceeds buffer size
    ResponseTooLarge,
    /// Response length is less than the minimum response length
    ResponseBadLen,
    /// Unknown response command
    ResponseUnknownCmd,
    /// Bad ACK in response
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
    /// Accumulator buffer is full
    AccumulateBufferFull,
    /// Operation not supported
    WontImplement,
    /// Operation not yet implemented
    NotYetImplemented,
    /// Serde serialization error
    SerdeSerCustom,
    /// Serde deserialization error
    SerdeDeCustom,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::RequestBufferFull => "Request buffer is full",
                Error::ResponseBadHeader => "Bad response header",
                Error::ResponseTooLarge => "Response exceeds buffer size",
                Error::ResponseBadLen => "Response length is less than the minimum response length",
                Error::ResponseUnknownCmd => "Unknown response command",
                Error::ResponseBadAck => "Bad ACK in response",
                Error::ResponseBadCrc => "Response CRC mismatch",
                Error::SerializeBadEnum => "Found an enum discriminant that was > u16::max_value()",
                Error::DeserializeUnexpectedEnd => "Hit the end of buffer, expected more data",
                Error::DeserializeBadBool => "Found a bool that wasn't 0 or 1",
                Error::DeserializeBadOption => "Found an Option discriminant that wasn't 0 or 1",
                Error::AccumulateBufferFull => "Accumulator buffer is full",
                Error::WontImplement => "Operation not supported",
                Error::NotYetImplemented => "Operation not yet implemented",
                Error::SerdeSerCustom => "Serde serialization error",
                Error::SerdeDeCustom => "Serde deserialization error",
            }
        )
    }
}

impl core::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        Error::SerdeSerCustom
    }
}

impl serde::de::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        Error::SerdeDeCustom
    }
}
