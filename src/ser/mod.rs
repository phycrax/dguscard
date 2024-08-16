pub(crate) mod serializer;
pub mod storage;

use crate::{
    error::Result,
    ser::{serializer::Serializer, storage::Slice},
    Command, Config,
};
use serde::Serialize;

#[cfg(feature = "heapless")]
use heapless::Vec;

/// Serialize a `T` to the given slice, with the resulting slice containing
/// data in a serialized format.
///
/// When successful, this function returns the slice containing the
/// serialized message
///
/// ## Example
///
/// ```rust
/// use postcard::to_slice;
/// let mut buf = [0u8; 32];
///
/// let used = to_slice(&true, &mut buf).unwrap();
/// assert_eq!(used, &[0x01]);
///
/// let used = to_slice("Hi!", &mut buf).unwrap();
/// assert_eq!(used, &[0x03, b'H', b'i', b'!']);
///
/// // NOTE: postcard handles `&[u8]` and `&[u8; N]` differently.
/// let data: &[u8] = &[0x01u8, 0x00, 0x20, 0x30];
/// let used = to_slice(data, &mut buf).unwrap();
/// assert_eq!(used, &[0x04, 0x01, 0x00, 0x20, 0x30]);
///
/// let data: &[u8; 4] = &[0x01u8, 0x00, 0x20, 0x30];
/// let used = to_slice(data, &mut buf).unwrap();
/// assert_eq!(used, &[0x01, 0x00, 0x20, 0x30]);
/// ```
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

/// Serialize a `T` to a `heapless::Vec<u8>`, with the `Vec` containing
/// data in a serialized then COBS encoded format. The terminating sentinel
/// `0x00` byte is included in the output `Vec`.
///
/// ## Example
///
/// ```rust
/// use postcard::to_vec_cobs;
/// use heapless::Vec;
/// use core::ops::Deref;
///
/// let ser: Vec<u8, 32> = to_vec_cobs(&false).unwrap();
/// assert_eq!(ser.deref(), &[0x01, 0x01, 0x00]);
///
/// let ser: Vec<u8, 32> = to_vec_cobs("Hi!").unwrap();
/// assert_eq!(ser.deref(), &[0x05, 0x03, b'H', b'i', b'!', 0x00]);
///
/// // NOTE: postcard handles `&[u8]` and `&[u8; N]` differently.
/// let data: &[u8] = &[0x01u8, 0x00, 0x20, 0x30];
/// let ser: Vec<u8, 32> = to_vec_cobs(data).unwrap();
/// assert_eq!(ser.deref(), &[0x03, 0x04, 0x01, 0x03, 0x20, 0x30, 0x00]);
///
/// let data: &[u8; 4] = &[0x01u8, 0x00, 0x20, 0x30];
/// let ser: Vec<u8, 32> = to_vec_cobs(data).unwrap();
/// assert_eq!(ser.deref(), &[0x02, 0x01, 0x03, 0x20, 0x30, 0x00]);
/// ```
#[cfg(feature = "heapless")]
#[cfg_attr(docsrs, doc(cfg(feature = "heapless")))]
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
