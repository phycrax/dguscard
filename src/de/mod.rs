pub(crate) mod deserializer;

use crate::error::Result;
use deserializer::Deserializer;
use serde::Deserialize;

pub fn from_bytes<'a, T>(input: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(input);
    T::deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct Params {
        energy: u16,
        freq: u16,
        counter: u32,
    }

    #[test]
    fn deserialize() {
        let input = [0, 10, 0, 5, 0x12, 0x34, 0x56, 0x78];
        let expected = Params {
            energy: 10,
            freq: 5,
            counter: 0x12345678,
        };
        let output: Params = from_bytes(&input).unwrap();
        assert_eq!(output, expected);
    }
}
