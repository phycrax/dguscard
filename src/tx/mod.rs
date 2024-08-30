mod serializer;
mod storage;

pub use storage::Storage;

use crate::{
    error::Result,
    tx::{serializer::Serializer, storage::Slice},
    Command, CRC, HEADER,
};
use serde::Serialize;

#[cfg(feature = "heapless")]
use heapless::Vec;

/// A TX frame builder with the capability of serializing values into itself.
/// 
/// Serialization output type is generic and must implement the [`Storage`] trait.
/// This trait is implemented for `[u8]` slice and `heapless::Vec`.
pub struct TxFrame<S: Storage> {
    serializer: Serializer<S>,
}

impl<'a> TxFrame<Slice<'a>> {
    /// Create a new frame that uses a slice as a given backing buffer
    /// The frame will be finalized as a slice
    pub fn with_slice(buf: &'a mut [u8], cmd: Command, addr: u16) -> Result<Self> {
        assert!(buf.len() >= 6, "Buffer too small");
        assert!(buf.len() <= u8::MAX as usize, "Buffer too large");
        Self::new(Slice::new(buf), cmd, addr)
    }
}

#[cfg(feature = "heapless")]
impl<const N: usize> TxFrame<Vec<u8, N>> {
    /// Create a new frame that uses [heapless::Vec] as an output
    /// The frame will be finalized as a [heapless::Vec]
    pub fn with_hvec(cmd: Command, addr: u16) -> Result<Self> {
        const {
            assert!(N >= 6, "Buffer too small");
            assert!(N <= u8::MAX as usize, "Buffer too large");
        };
        Self::new(Vec::new(), cmd, addr)
    }
}

impl<S: Storage<Output = O>, O> TxFrame<S> {
    /// Create a new frame with an output type that implements [`Storage`] trait.
    /// The frame will be finalized as the given output type.
    /// It should rarely be necessary to directly use this type unless you implemented your own [`Storage`].
    pub fn new(output: S, cmd: Command, addr: u16) -> Result<Self> {
        let mut serializer = Serializer { output };
        HEADER.serialize(&mut serializer)?;
        (cmd as u16).serialize(&mut serializer)?;
        addr.serialize(&mut serializer)?;
        Ok(Self { serializer })
    }

    /// Copy and serialize a `T` into the frame
    pub fn copy_from<T: Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut self.serializer)
    }
    
    /// Finalize the frame with optional crc and get the output
    pub fn finalize(mut self, crc: bool) -> Result<O> {
        if crc {
            let crc = CRC.checksum(&self.serializer.output[3..]).swap_bytes();
            crc.serialize(&mut self.serializer)?;
        }
        self.serializer.output[2] = self.serializer.output.len() as u8 - 3;
        Ok(self.serializer.output.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let mut frame = TxFrame::with_slice(buf, Command::WriteVp, 0x00DE).unwrap();
        frame.copy_from(&data).unwrap();
        let output = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_slice_nocrc() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let data = TestTuple::new();

        let mut frame = TxFrame::with_slice(buf, Command::WriteVp, 0x00DE).unwrap();
        frame.copy_from(&data).unwrap();
        let output = frame.finalize(false).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_hvec() {
        let expected: Vec<u8, 12> = Vec::from_slice(&[
            0x5A, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0E, 0xB4,
        ])
        .unwrap();
        let data = TestTuple::new();

        let mut frame = TxFrame::with_hvec(Command::WriteVp, 0x00DE).unwrap();
        frame.copy_from(&data).unwrap();
        let output: Vec<u8, 12> = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_hvec_nocrc() {
        let expected: Vec<u8, 10> =
            Vec::from_slice(&[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34]).unwrap();
        let data = TestTuple::new();

        let mut frame = TxFrame::with_hvec(Command::WriteVp, 0x00DE).unwrap();
        frame.copy_from(&data).unwrap();
        let output: Vec<u8, 10> = frame.finalize(false).unwrap();
        assert_eq!(output, expected);
    }
}
