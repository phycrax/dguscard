//! Request frame builder/serializer

mod serializer;
mod storage;

pub use storage::Storage;

use crate::{error::Result, CRC, HEADER};
use serde::Serialize;
use serializer::Serializer;
use storage::Slice;

#[cfg(feature = "heapless")]
use heapless::Vec;

/// Request Instruction
///
/// Refer to T5L_DGUS2 DevGuide Section 4.2
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(missing_docs)]
pub enum RequestInstruction {
    /// Read data from register
    ReadReg { page: u8, addr: u8, len: u8 },
    /// Read word data from variable space, using word address
    ReadWord { addr: u16, len: u8 },
    /// Read double word data from variable space, using double word address
    ReadDword { addr: u32, len: u8 },
    /// Write data to register
    WriteReg { page: u8, addr: u8 },
    /// Write word data to variable space, using word address
    WriteWord { addr: u16 },
    /// Write double word data to variable space, using double word address
    WriteDword { addr: u32 },
    /// Write curve buffer data to variable space, using channel number
    WriteCurve { ch: u8 },
}

/// Request frame builder
///
/// Serialization output type is generic and must implement the [`Storage`] trait.
/// This trait is implemented for [`u8`] slice and [`heapless::Vec`].
///
/// # Examples
///
/// ```rust
/// use dguscard::request::{RequestFrame, RequestInstruction};
/// # use std::io::Write;
/// #[derive(serde::Serialize)]
/// struct MyData {
///     byte: u8,
///     word: u16,
///     dword: u32,
/// }
/// let data = MyData { byte: 11, word: 2222, dword: 333333 };
///
/// let mut uart = /* Anything that implements the `Write` trait */
/// # Vec::new();
/// // Backing buffer for the frame.
/// let buf = &mut [0u8; 50];
/// // Construct a frame with the slice buffer/output type and write data instruction.
/// let mut frame = RequestFrame::with_slice(buf, RequestInstruction::WriteWord { addr: 0x1234 }).unwrap();
/// // Push the data into the frame.
/// frame.push(&data).unwrap();
/// // It's possible to push multiple different data types into the frame.
/// frame.push(&[1u8,2,3,4]).unwrap();
/// // Finalize the frame with CRC and get the output.
/// let tx_bytes = frame.finalize(true).unwrap();
/// // Transmit the frame
/// uart.write_all(tx_bytes).unwrap();
/// ```
///
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RequestFrame<S: Storage> {
    serializer: Serializer<S>,
}

impl<'a> RequestFrame<Slice<'a>> {
    /// Constructs a new frame that uses a slice as a given backing buffer.
    /// The frame will be finalized as a slice.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is too small or too large to contain a frame
    pub fn with_slice(buf: &'a mut [u8], instr: RequestInstruction) -> Result<Self> {
        assert!(buf.len() >= 6, "Buffer too small");
        assert!(buf.len() <= u8::MAX as usize, "Buffer too large");
        Self::new(Slice::new(buf), instr)
    }
}

#[cfg(feature = "heapless")]
impl<const N: usize> RequestFrame<Vec<u8, N>> {
    /// Constructs a new frame that uses [`heapless::Vec`] as an output.
    /// The frame will be finalized as a [`heapless::Vec`].
    pub fn with_hvec(instr: RequestInstruction) -> Result<Self> {
        const {
            assert!(N >= 6, "Buffer too small");
            assert!(N <= u8::MAX as usize, "Buffer too large");
        };
        Self::new(Vec::new(), instr)
    }
}

impl<S: Storage<Output = O>, O> RequestFrame<S> {
    /// Constructs a new frame with an output type that implements [`Storage`] trait.
    /// The frame will be finalized as the given output type.
    /// It should rarely be necessary to directly use this function unless you implemented your own [`Storage`].
    pub fn new(output: S, instr: RequestInstruction) -> Result<Self> {
        let mut serializer = Serializer { output };
        // Push header
        HEADER.serialize(&mut serializer)?;
        // Push length placeholder
        serializer.output.try_push(0u8)?;
        // Push instruction
        use RequestInstruction::*;
        match instr {
            WriteReg { page, addr } => {
                serializer.output.try_push(0x80)?;
                serializer.output.try_push(page)?;
                serializer.output.try_push(addr)?;
            }

            ReadReg { page, addr, len } => {
                serializer.output.try_push(0x81)?;
                serializer.output.try_push(page)?;
                serializer.output.try_push(addr)?;
                serializer.output.try_push(len)?;
            }

            WriteWord { addr } => {
                serializer.output.try_push(0x82)?;
                addr.serialize(&mut serializer)?;
            }

            ReadWord { addr, len } => {
                serializer.output.try_push(0x83)?;
                addr.serialize(&mut serializer)?;
                serializer.output.try_push(len)?;
            }

            WriteCurve { ch } => {
                serializer.output.try_push(0x84)?;
                serializer.output.try_push(ch)?;
            }

            WriteDword { addr } => {
                serializer.output.try_push(0x86)?;
                addr.serialize(&mut serializer)?;
            }

            ReadDword { addr, len } => {
                serializer.output.try_push(0x87)?;
                addr.serialize(&mut serializer)?;
                serializer.output.try_push(len)?;
            }
        }
        // Return the frame
        Ok(Self { serializer })
    }

    /// Appends a `T` into the frame
    pub fn push<T: Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut self.serializer)
    }

    /// Finalizes the frame with optional CRC and returns the output
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
    fn tuple_to_slice_crc() {
        let buf = &mut [0u8; 20];
        let expected = &[
            0x5A, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0E, 0xB4,
        ];
        let data = TestTuple::new();

        let mut frame =
            RequestFrame::with_slice(buf, RequestInstruction::WriteWord { addr: 0x00DE }).unwrap();
        frame.push(&data).unwrap();
        let output = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_slice_nocrc() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let data = TestTuple::new();

        let mut frame =
            RequestFrame::with_slice(buf, RequestInstruction::WriteWord { addr: 0x00DE }).unwrap();
        frame.push(&data).unwrap();
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

        let mut frame =
            RequestFrame::with_hvec(RequestInstruction::WriteWord { addr: 0x00DE }).unwrap();
        frame.push(&data).unwrap();
        let output: Vec<u8, 12> = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_hvec_nocrc() {
        let expected: Vec<u8, 10> =
            Vec::from_slice(&[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34]).unwrap();
        let data = TestTuple::new();

        let mut frame =
            RequestFrame::with_hvec(RequestInstruction::WriteWord { addr: 0x00DE }).unwrap();
        frame.push(&data).unwrap();
        let output: Vec<u8, 10> = frame.finalize(false).unwrap();
        assert_eq!(output, expected);
    }
}
