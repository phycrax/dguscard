//! Request building

mod serializer;
mod storage;

pub use self::storage::{Slice, Storage};

use self::serializer::Serializer;
use crate::{Instruction, Result, Write, CRC, HEADER};
use core::marker::PhantomData;
use serde::Serialize;

#[cfg(feature = "heapless")]
use heapless::Vec;

/// Request builder
///
/// Output type is generic and must implement the [`Storage`] trait.
/// This trait is implemented for [`u8`] slice and [`heapless::Vec<u8>`].
///
/// # Examples
///
/// ```rust
/// use dguscard::{request::Request, Word, Write};
/// # use std::io::Write as IoWrite;
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
/// let mut frame = Request::with_slice(buf, Word { addr: 0x1234, cmd: Write }).unwrap();
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
pub struct Request<C, S: Storage> {
    serializer: Serializer<S>,
    cmd: PhantomData<C>,
}

impl<'a, C> Request<C, Slice<'a>> {
    /// Constructs a new frame that uses a slice as a given backing buffer.
    /// The frame will be finalized as a slice.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is too small or too large to contain a frame
    pub fn with_slice(buf: &'a mut [u8], instr: impl Instruction) -> Result<Self> {
        Self::new_inner(Slice::new(buf), instr)
    }
}

#[cfg(feature = "heapless")]
impl<C, const N: usize> Request<C, Vec<u8, N>> {
    /// Constructs a new frame that uses [`heapless::Vec`] as an output.
    /// The frame will be finalized as a [`heapless::Vec`].
    pub fn with_hvec(instr: impl Instruction) -> Result<Self> {
        Self::new_inner(Vec::new(), instr)
    }
}

impl<C, S, O> Request<C, S>
where
    S: Storage<Output = O>,
{
    /// Constructs a new frame with an output type that implements [`Storage`] trait.
    /// The frame will be finalized as the given output type.
    /// It should rarely be necessary to directly use this function unless you implemented your own [`Storage`].
    fn new_inner<I: Instruction>(output: S, instr: I) -> Result<Self> {
        let mut serializer = Serializer { output };
        // Push header
        HEADER.serialize(&mut serializer)?;
        // Push length placeholder
        0u8.serialize(&mut serializer)?;
        // Push instruction
        I::OPCODE.serialize(&mut serializer)?;
        instr.serialize(&mut serializer)?;
        // Return the frame
        Ok(Self {
            serializer,
            cmd: PhantomData,
        })
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

impl<S, O> Request<Write, S>
where
    S: Storage<Output = O>,
{
    /// Appends a `T` into the frame
    pub fn push<T: Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut self.serializer)
    }
}

#[cfg(test)]
mod tests {
    use crate::Word;

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

        let mut frame = Request::with_slice(
            buf,
            Word {
                addr: 0x00DE,
                cmd: Write,
            },
        )
        .unwrap();
        frame.push(&data).unwrap();
        let output = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_slice_nocrc() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let data = TestTuple::new();

        let mut frame = Request::with_slice(
            buf,
            Word {
                addr: 0x00DE,
                cmd: Write,
            },
        )
        .unwrap();
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

        let mut frame = Request::with_hvec(Word {
            addr: 0x00DE,
            cmd: Write,
        })
        .unwrap();
        frame.push(&data).unwrap();
        let output: Vec<u8, 12> = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_hvec_nocrc() {
        let expected: Vec<u8, 10> =
            Vec::from_slice(&[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34]).unwrap();
        let data = TestTuple::new();

        let mut frame = Request::with_hvec(Word {
            addr: 0x00DE,
            cmd: Write,
        })
        .unwrap();
        frame.push(&data).unwrap();
        let output: Vec<u8, 10> = frame.finalize(false).unwrap();
        assert_eq!(output, expected);
    }
}
