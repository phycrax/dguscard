pub(crate) mod deserializer;
pub mod parser;

use crate::{
    de::deserializer::Deserializer,
    de::parser::{Metadata, DataBytes},
    error::{Error, Result},
    Command, Config,
};
use serde::Deserialize;

/// Deserialize a message of type `T` from a data byte slice.
/// The unused portion (if any) of the byte slice is not returned.
pub fn from_bytes<'a, T>(input: DataBytes<'a>) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(input.0);
    T::deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;
}
