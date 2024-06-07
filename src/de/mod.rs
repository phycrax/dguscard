use crate::{
    error::{Error, Result},
    DwinVariable,
};
use deserializer::Deserializer;
use serde::Deserialize;

pub(crate) mod deserializer;

pub fn from_bytes<'a, T>(input: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a> + DwinVariable,
{
    let mut deserializer = Deserializer::from_bytes(input);
    T::deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct BackgroundIcl(u16, u16);

    impl BackgroundIcl {
        pub fn new(id: u16) -> Self {
            Self(0x5A00, id)
        }
    }

    impl DwinVariable for BackgroundIcl {
        const ADDRESS: u16 = 0x00DE;
    }

    #[test]
    fn deserialize() {
        let input = [0x5A, 0x00, 0x12, 0x34];
        let expected = BackgroundIcl::new(0x1234);
        let output: BackgroundIcl = from_bytes(&input).unwrap();
        assert_eq!(output, expected);
    }
}
