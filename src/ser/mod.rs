pub(crate) mod serializer;
pub mod storage;


use crate::{
    error::Result,
    ser::{
        serializer::Serializer,
        storage::{Slice, Storage},
    },
    Command, CRC,
};
use serde::Serialize;

#[cfg(feature = "heapless")]
use heapless::Vec;

pub struct Frame<S: Storage> {
    pub serializer: Serializer<S>,
}

impl<S: Storage<Output = O>, O> Frame<S> {
    pub fn new(mut serializer: Serializer<S>, cmd: Command, addr: u16) -> Result<Self> {
        0x5AA5u16.serialize(&mut serializer)?;
        (cmd as u16).serialize(&mut serializer)?;
        addr.serialize(&mut serializer)?;
        Ok(Self { serializer })
    }
    
    pub fn copy_from<T: Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut self.serializer)
    }

    pub fn finalize(mut self, crc: bool) -> Result<O> {
        if crc {
            let crc = CRC.checksum(&self.serializer.output[3..]).swap_bytes();
            crc.serialize(&mut self.serializer)?;
        }
        self.serializer.output[2] = self.serializer.output.len() as u8 - 3;
        Ok(self.serializer.output.finalize())
    }
}

impl<'a> Frame<Slice<'a>> {
    pub fn with_slice(buf: &'a mut [u8], cmd: Command, addr: u16) -> Result<Self> {
        Self::new(
            Serializer {
                output: Slice::new(buf),
            },
            cmd,
            addr,
        )
    }
}

#[cfg(feature = "heapless")]
impl<const N: usize> Frame<Vec<u8, N>> {
    pub fn with_hvec(cmd: Command, addr: u16) -> Result<Self> {
        Self::new(Serializer { output: Vec::new() }, cmd, addr)
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

        let mut frame = Frame::with_slice(buf, Command::WriteVp, 0x00DE).unwrap();
        frame.copy_from(&data).unwrap();
        let output = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_slice_nocrc() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let data = TestTuple::new();

        let mut frame = Frame::with_slice(buf, Command::WriteVp, 0x00DE).unwrap();
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

        let mut frame = Frame::with_hvec(Command::WriteVp, 0x00DE).unwrap();
        frame.copy_from(&data).unwrap();
        let output: Vec<u8, 12> = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_hvec_nocrc() {
        let expected: Vec<u8, 10> =
            Vec::from_slice(&[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34]).unwrap();
        let data = TestTuple::new();

        let mut frame = Frame::with_hvec(Command::WriteVp, 0x00DE).unwrap();
        frame.copy_from(&data).unwrap();
        let output: Vec<u8, 10> = frame.finalize(false).unwrap();
        assert_eq!(output, expected);
    }
}
