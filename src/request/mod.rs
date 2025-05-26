//! Request builder

mod serializer;
mod storage;

pub use self::storage::{Slice, Storage};

#[cfg(feature = "heapless")]
pub use self::storage::HVec;

use self::serializer::Serializer;
use crate::{
    command::{Command, Write},
    Result, CRC, HEADER,
};
use core::marker::PhantomData;
use serde::Serialize;

/// Request builder
///
/// Output type is generic and must implement the [`Storage`] trait.
/// This trait is implemented for [`Slice`]([`u8`] slice newtype)
/// and [`HVec`]([`Vec<u8, N>`][heapless::Vec] newtype).
///
/// # Example
///
/// ```rust
/// use dguscard::{request::Request, command::{Word, Write}};
/// # use std::io::Write as IoWrite;
/// #[derive(serde::Serialize)]
/// struct MyData {
///     byte_h: u8,
///     byte_l: u8,
///     word: u16,
///     dword: u32,
///     float: f32,
///     double: f64,
/// }
/// let data = MyData { byte_h: 0, byte_l: 1, word: 2, dword: 3, float: 4.0, double: 5.0 };
///
/// let mut uart =
/// # Vec::new();
/// // Backing buffer for the request.
/// let buf = &mut [0u8; 50];
/// // Get a request builder with the slice buffer/output type and write word command.
/// let mut frame = Request::with_slice(buf, Word { addr: 0x1234, cmd: Write}).unwrap();
/// // Push your data into the request.
/// frame.push(&data).unwrap();
/// // It's possible to push multiple different data types into the request.
/// // As long as they implement `Serialize` and are compatible with the data model.
/// frame.push(&[1u8,2,3,4]).unwrap();
/// // Finalize the request with CRC and get the output.
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
    /// Returns a new builder that uses a [`Slice`] as a given backing buffer.
    /// The request will be finalized as [`u8`] slice.
    pub fn with_slice(buf: &'a mut [u8], cmd: impl Command) -> Result<Self> {
        Self::new(Slice::new(buf), cmd)
    }
}

#[cfg(feature = "heapless")]
impl<C, const N: usize> Request<C, HVec<N>> {
    /// Returns a new builder that uses [`HVec`] as a buffer.
    /// The request will be finalized as [`Vec<u8, N>`][heapless::Vec].
    pub fn with_hvec(cmd: impl Command) -> Result<Self> {
        Self::new(HVec::new(), cmd)
    }
}

impl<S, O> Request<Write, S>
where
    S: Storage<Output = O>,
{
    /// Appends a `T` into the [`Request<Write, S>`].
    pub fn push<T: Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut self.serializer)
    }
}

impl<RW, S, O> Request<RW, S>
where
    S: Storage<Output = O>,
{
    /// Returns a new builder with an output type that implements [`Storage`] trait.
    /// The request will be finalized as the given output type.
    /// It should rarely be necessary to directly use this function unless you implemented your own [`Storage`].
    pub fn new<C: Command>(output: S, cmd: C) -> Result<Self> {
        let mut serializer = Serializer { output };
        // Push header
        HEADER.serialize(&mut serializer)?;
        // Push length placeholder
        0u8.serialize(&mut serializer)?;
        // Push command code
        C::CMD.serialize(&mut serializer)?;
        // Push command data
        cmd.serialize(&mut serializer)?;
        // Return the builder
        Ok(Self {
            serializer,
            cmd: PhantomData,
        })
    }

    /// Finalizes the request with optional CRC and returns the output.
    pub fn finalize(mut self, crc: bool) -> Result<O> {
        if crc {
            let crc = CRC.checksum(&self.serializer.output[3..]).swap_bytes();
            crc.serialize(&mut self.serializer)?;
        }
        self.serializer.output[2] = self.serializer.output.len() as u8 - 3;
        Ok(self.serializer.output.finalize())
    }
}

#[cfg(feature = "heapless")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Word, Write};
    use heapless::Vec;

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
