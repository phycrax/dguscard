//! Request builder

mod serializer;
mod storage;

pub use self::storage::{Slice, Storage};

#[cfg(feature = "heapless")]
pub use self::storage::HVec;

use self::serializer::Serializer;
use crate::{Address, CRC, HEADER, Result, Sealed};
use core::marker::PhantomData;
use serde::Serialize;

/// Request builder
///
/// Output type is generic and must implement the [`Storage`] trait.
/// This trait is implemented for [`Slice`]([`u8`] slice newtype)
/// and [`HVec`]([`Vec<u8, N>`][heapless::Vec] newtype).
///
/// A request is built from an [`Address`] and a command ([`Write`] or
/// [`Read`]). Only a [`Request<Write, S>`] can be pushed
/// into, so a read request cannot carry data.
///
/// # Example
///
/// ```rust
/// use dguscard::{request::{Request, Write}, Address};
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
/// // Get a request builder with the slice buffer/output type, writing to a variable address.
/// let mut frame = Request::with_slice(buf, Address::Word(0x1234), Write).unwrap();
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
/// Reading instead, which takes no data:
///
/// ```rust
/// use dguscard::{request::{Request, Read}, Address};
/// let buf = &mut [0u8; 50];
/// // Read 4 words from the variable address 0x1234.
/// let frame = Request::with_slice(buf, Address::Word(0x1234), Read(4))
///     .unwrap()
///     .finalize(true)
///     .unwrap();
/// ```
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Request<C, S: Storage> {
    serializer: Serializer<S>,
    cmd: PhantomData<C>,
}

impl<'a, C: Command> Request<C, Slice<'a>> {
    /// Returns a new builder that uses a [`Slice`] as a given backing buffer.
    /// The request will be finalized as [`u8`] slice.
    pub fn with_slice(buf: &'a mut [u8], addr: Address, cmd: C) -> Result<Self> {
        Self::new(Slice::new(buf), addr, cmd)
    }
}

#[cfg(feature = "heapless")]
impl<C: Command, const N: usize> Request<C, HVec<N>> {
    /// Returns a new builder that uses [`HVec`] as a buffer.
    /// The request will be finalized as [`Vec<u8, N>`][heapless::Vec].
    pub fn with_hvec(addr: Address, cmd: C) -> Result<Self> {
        Self::new(HVec::new(), addr, cmd)
    }
}

impl<S, O> Request<Write, S>
where
    S: Storage<Output = O>,
{
    /// Appends a `T` into the [`Request<Write, S>`].
    pub fn push<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut self.serializer)
    }
}

impl<C, S, O> Request<C, S>
where
    C: Command,
    S: Storage<Output = O>,
{
    /// Returns a new builder with an output type that implements [`Storage`] trait.
    /// The request will be finalized as the given output type.
    /// It should rarely be necessary to directly use this function unless you implemented your own [`Storage`].
    pub fn new(output: S, addr: Address, cmd: C) -> Result<Self> {
        let mut serializer = Serializer { output };
        // Push header
        HEADER.serialize(&mut serializer)?;
        // Push length placeholder
        0u8.serialize(&mut serializer)?;
        // Push command code
        addr.code(cmd.wlen().is_some()).serialize(&mut serializer)?;
        // Push address
        match addr {
            Address::Register(page, addr) => {
                page.serialize(&mut serializer)?;
                addr.serialize(&mut serializer)?;
            }
            Address::Word(addr) => addr.serialize(&mut serializer)?,
            Address::Dword(addr) => addr.serialize(&mut serializer)?,
            Address::Curve(ch_mode) => ch_mode.serialize(&mut serializer)?,
        }
        // Push word length if it's read
        if let Some(wlen) = cmd.wlen() {
            wlen.serialize(&mut serializer)?;
        }
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
        self.serializer.output[2] = (self.serializer.output.len() - 3) as u8;
        Ok(self.serializer.output.finalize())
    }
}

/// Write command
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Write;

/// Read command with word length
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Read(pub u8);

/// Command trait (sealed)
///
/// Implemented by all commands. Users are responsible for valid command parameters such as address and length.
/// - A [`Request`] with the command [`Read`] will be responded with [`Data`][crate::response::Response::Data].
/// - A [`Request`] with the command [`Write`] will responded with [`Ack`][crate::response::Response::Ack].
#[allow(private_bounds)]
pub trait Command: Sealed {
    /// Returns the word length for read commands, or `None` for write commands.
    fn wlen(&self) -> Option<u8>;
}

impl Sealed for Write {}
impl Command for Write {
    fn wlen(&self) -> Option<u8> {
        None
    }
}

impl Sealed for Read {}
impl Command for Read {
    fn wlen(&self) -> Option<u8> {
        Some(self.0)
    }
}

#[cfg(feature = "heapless")]
#[cfg(test)]
mod tests {
    use super::*;
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

        let mut frame = Request::with_slice(buf, Address::Word(0x00DE), Write).unwrap();
        frame.push(&data).unwrap();
        let output = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_slice_nocrc() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let data = TestTuple::new();

        let mut frame = Request::with_slice(buf, Address::Word(0x00DE), Write).unwrap();
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

        let mut frame = Request::with_hvec(Address::Word(0x00DE), Write).unwrap();
        frame.push(&data).unwrap();
        let output: Vec<u8, 12> = frame.finalize(true).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_hvec_nocrc() {
        let expected: Vec<u8, 10> =
            Vec::from_slice(&[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34]).unwrap();
        let data = TestTuple::new();

        let mut frame = Request::with_hvec(Address::Word(0x00DE), Write).unwrap();
        frame.push(&data).unwrap();
        let output: Vec<u8, 10> = frame.finalize(false).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn read_word() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 4, 0x83, 0x12, 0x34, 4];

        let output = Request::with_slice(buf, Address::Word(0x1234), Read(4))
            .unwrap()
            .finalize(false)
            .unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn read_register() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 4, 0x81, 0x01, 0x02, 3];

        let output = Request::with_slice(buf, Address::Register(0x01, 0x02), Read(3))
            .unwrap()
            .finalize(false)
            .unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn read_dword() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 6, 0x87, 0x00, 0x01, 0x02, 0x03, 2];

        let output = Request::with_slice(buf, Address::Dword(0x0001_0203), Read(2))
            .unwrap()
            .finalize(false)
            .unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn write_curve() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 4, 0x84, 0x01, 0x12, 0x34];

        let mut frame = Request::with_slice(buf, Address::Curve(0x01), Write).unwrap();
        frame.push(&0x1234u16).unwrap();
        let output = frame.finalize(false).unwrap();
        assert_eq!(output, expected);
    }

    // Compile test: push should not be available on a Read request
    /// ```compile_fail
    /// let buf = &mut [0u8; 20];
    /// let mut frame = Request::with_slice(buf, Address::Word(0), Read(0)).unwrap();
    /// frame.push(&42u8).unwrap(); // push is not available on a Read request
    /// ```
    #[test]
    fn read_request_push_should_not_compile() {}
}
