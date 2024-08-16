pub(crate) mod serializer;
pub(crate) mod storage;

use crate::{
    error::Result,
    ser::{serializer::Serializer, storage::Slice},
    Command, Config,
};
use serde::Serialize;

#[cfg(feature = "heapless")]
use heapless::Vec;

pub fn to_slice<'b, T>(
    value: &T,
    buf: &'b mut [u8],
    addr: u16,
    cmd: Command,
    cfg: Config,
) -> Result<&'b mut [u8]>
where
    T: Serialize,
{
    let mut serializer = Serializer::new(Slice::new(buf), cfg.header, cmd, addr)?;
    value.serialize(&mut serializer)?;
    serializer.finalize(cfg.crc)
}

#[cfg(feature = "heapless")]
pub fn to_hvec<const N: usize, T>(
    value: &T,
    addr: u16,
    cmd: Command,
    cfg: Config,
) -> Result<Vec<u8, N>>
where
    T: Serialize,
{
    let mut serializer = Serializer::new(Vec::new(), cfg.header, cmd, addr)?;
    value.serialize(&mut serializer)?;
    serializer.finalize(cfg.crc)
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::error::{Error, Result};

    #[derive(Serialize)]
    struct TestTuple(u16, u16);

    impl TestTuple {
        pub fn new() -> Self {
            Self(0x5A00, 0x1234)
        }
    }

    #[test]
    fn test_tuple() {
        let expected = [
            0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
        ];
        let expected: Vec<u8, 12> = Vec::from_slice(&expected).unwrap();
        let data = TestTuple::new();

        let output: Vec<u8, 12> =
            to_hvec(&data, 0x00DE, Command::Write, Default::default()).unwrap();
        assert_eq!(output, expected);
    }
}
