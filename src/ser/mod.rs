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
        fn new() -> Self {
            Self(0x5A00, 0x1234)
        }
    }

    #[test]
    fn tuple_to_slice() {
        let buf = &mut [0u8; 20];
        let expected = &[
            0x5A, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0E, 0xB4,
        ];
        let data = TestTuple::new();
        let cfg = Config {
            ..Default::default()
        };

        let output = to_slice(&data, buf, 0x00DE, Command::Write, cfg).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_slice_nocrc() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let data = TestTuple::new();
        let cfg = Config {
            crc: None,
            ..Default::default()
        };

        let output = to_slice(&data, buf, 0x00DE, Command::Write, cfg).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_hvec() {
        let expected: Vec<u8, 12> = Vec::from_slice(&[
            0x5A, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0E, 0xB4,
        ])
        .unwrap();
        let data = TestTuple::new();
        let cfg = Config {
            ..Default::default()
        };

        let output: Vec<_, 12> = to_hvec(&data, 0x00DE, Command::Write, cfg).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_hvec_nocrc() {
        let expected: Vec<u8, 10> =
            Vec::from_slice(&[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34]).unwrap();
        let data = TestTuple::new();
        let cfg = Config {
            crc: None,
            ..Default::default()
        };

        let output: Vec<_, 10> = to_hvec(&data, 0x00DE, Command::Write, cfg).unwrap();
        assert_eq!(output, expected);
    }
}
