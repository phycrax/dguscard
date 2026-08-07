//! Request builder

mod serializer;
mod storage;

use core::fmt::Formatter;
use core::marker::PhantomData;

use serde::Serialize;

use self::serializer::Serializer;
#[cfg(feature = "heapless")]
pub use self::storage::HVec;
pub use self::storage::{Slice, Storage};
use crate::{Address, Checksum, Crc, FRAME_OVERHEAD, HEADER, MAX_CONTENT_LEN, Sealed};

/// Errors that can occur while building a request
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Request buffer is full
    BufferFull,
    /// Request content is longer than the frame length byte can describe
    OversizedFrame,
    /// Found an enum discriminant that was > [`u16::MAX`]
    BadEnum,
    /// The type is not representable in the DGUS wire format
    Unsupported,
    /// Serde serialization error
    Serde,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::BufferFull => "Request buffer is full",
                Error::OversizedFrame => "Request content is longer than the frame length byte can describe",
                Error::BadEnum => "Found an enum discriminant that was > u16::MAX",
                Error::Unsupported => "The type is not representable in the DGUS wire format",
                Error::Serde => "Serde serialization error",
            }
        )
    }
}

impl core::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        Error::Serde
    }
}

/// Request builder
///
/// Output type is generic and must implement the [`Storage`] trait.
/// This trait is implemented for [`Slice`]([`u8`] slice newtype)
/// and [`HVec`]([`Vec<u8, N>`][heapless::Vec] newtype).
///
/// A request is built from an [`Address`] and a command ([`Write`] or
/// [`Read`]). Only a [`Request<Write, S, K>`] can be pushed
/// into, so a read request cannot carry data.
///
/// The builder is given its checksum up front — [`Crc::on()`], [`Crc::off()`],
/// or `Some(&mut ..)` for your own [`Checksum`].
///
/// # Example
///
/// ```rust
/// use dguscard::{request::{Request, Write}, Address, Crc};
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
/// // Get a request builder with the slice buffer/output type, writing to a
/// // variable address, with CRC enabled.
/// let mut frame = Request::with_slice(buf, Address::Word(0x1234), Write, Crc::on()).unwrap();
/// // Push your data into the request.
/// frame.push(&data).unwrap();
/// // It's possible to push multiple different data types into the request.
/// // As long as they implement `Serialize` and are compatible with the data model.
/// frame.push(&[1u8,2,3,4]).unwrap();
/// // Finalize the request and get the output.
/// let tx_bytes = frame.finalize().unwrap();
/// // Transmit the frame
/// uart.write_all(tx_bytes).unwrap();
/// ```
///
/// Reading instead, which takes no data:
///
/// ```rust
/// use dguscard::{request::{Request, Read}, Address, Crc};
/// let buf = &mut [0u8; 50];
/// // Read 4 words from the variable address 0x1234.
/// let frame = Request::with_slice(buf, Address::Word(0x1234), Read(4), Crc::on())
///     .unwrap()
///     .finalize()
///     .unwrap();
/// ```
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Request<C, S: Storage, K: Checksum = Crc> {
    serializer: Serializer<S>,
    crc: Option<K>,
    cmd: PhantomData<C>,
}

impl<'a, C: Command, K: Checksum> Request<C, Slice<'a>, K> {
    /// Returns a new builder that uses a [`Slice`] as a given backing buffer.
    /// The request will be finalized as [`u8`] slice.
    ///
    /// Pass [`Crc::on()`] or [`Crc::off()`] for `crc`, or `Some(&mut ..)` to
    /// lend your own [`Checksum`] implementation.
    pub fn with_slice(buf: &'a mut [u8], addr: Address, cmd: C, crc: Option<K>) -> Result<Self, Error> {
        Self::new(Slice::new(buf), addr, cmd, crc)
    }
}

#[cfg(feature = "heapless")]
impl<C: Command, K: Checksum, const N: usize> Request<C, HVec<N>, K> {
    /// Returns a new builder that uses [`HVec`] as a buffer.
    /// The request will be finalized as [`Vec<u8, N>`][heapless::Vec].
    ///
    /// Pass [`Crc::on()`] or [`Crc::off()`] for `crc`, or `Some(&mut ..)` to
    /// lend your own [`Checksum`] implementation.
    pub fn with_hvec(addr: Address, cmd: C, crc: Option<K>) -> Result<Self, Error> {
        Self::new(HVec::new(), addr, cmd, crc)
    }
}

impl<S, O, K> Request<Write, S, K>
where
    S: Storage<Output = O>,
    K: Checksum,
{
    /// Appends a `T` into the [`Request<Write, S, K>`].
    pub fn push<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut self.serializer)
    }
}

impl<C, S, O, K> Request<C, S, K>
where
    C: Command,
    S: Storage<Output = O>,
    K: Checksum,
{
    /// Returns a new builder with an output type that implements [`Storage`] trait.
    /// The request will be finalized as the given output type.
    /// It should rarely be necessary to directly use this function unless you implemented your own [`Storage`].
    pub fn new(output: S, addr: Address, cmd: C, crc: Option<K>) -> Result<Self, Error> {
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
            crc,
            cmd: PhantomData,
        })
    }

    /// Finalizes the request, appending the checksum if the builder was given
    /// one, and returns the output.
    pub fn finalize(mut self) -> Result<O, Error> {
        if let Some(crc) = self.crc.as_mut() {
            let sum = crc.compute(&self.serializer.output[FRAME_OVERHEAD..]);
            self.serializer.output.try_extend(&sum.to_le_bytes())?;
        }
        let content_len = self.serializer.output.len() - FRAME_OVERHEAD;
        if content_len > MAX_CONTENT_LEN {
            return Err(Error::OversizedFrame);
        }
        self.serializer.output[2] = content_len as u8;
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

#[cfg(test)]
mod tests {
    #[cfg(feature = "heapless")]
    use heapless::Vec;

    use super::*;

    #[derive(Serialize)]
    struct TestTuple(u16, u16);

    impl TestTuple {
        fn new() -> Self {
            Self(0x5A00, 0x1234)
        }
    }

    #[test]
    fn frame_fills_length_byte() {
        let buf = &mut [0u8; crate::MAX_FRAME_LEN];
        let mut frame = Request::with_slice(buf, Address::Word(0x1234), Write, Crc::off()).unwrap();
        // Command and address take 3 content bytes, so 126 words saturate the
        // length byte exactly.
        for _ in 0..126 {
            frame.push(&0xABCDu16).unwrap();
        }

        let output = frame.finalize().unwrap();
        assert_eq!(output.len(), crate::MAX_FRAME_LEN);
        assert_eq!(output[2], MAX_CONTENT_LEN as u8);
    }

    #[test]
    fn frame_cannot_outgrow_length_byte() {
        let buf = &mut [0u8; crate::MAX_FRAME_LEN];
        let mut frame = Request::with_slice(buf, Address::Word(0x1234), Write, Crc::off()).unwrap();
        for _ in 0..126 {
            frame.push(&0xABCDu16).unwrap();
        }

        // This buffer is exactly one frame long, so running out of room is what
        // stops the frame growing, before `finalize` gets to measure it.
        assert_eq!(frame.push(&0xABCDu16), Err(Error::BufferFull));
    }

    #[test]
    fn oversized_frame_errors_at_finalize() {
        // A larger-than-a-frame buffer is the caller's business, so it is
        // accepted; the frame it describes is what has to stay in range.
        let buf = &mut [0u8; crate::MAX_FRAME_LEN + 8];
        let mut frame = Request::with_slice(buf, Address::Word(0x1234), Write, Crc::off()).unwrap();
        // One word past what the length byte can describe: 3 + 127 * 2 = 257.
        for _ in 0..127 {
            frame.push(&0xABCDu16).unwrap();
        }

        assert_eq!(frame.finalize(), Err(Error::OversizedFrame));
    }

    #[test]
    fn oversized_frame_counts_the_crc() {
        // 255 content bytes fit, but only without a trailer: the checksum is
        // appended before the length is measured, so this overflows by two.
        let buf = &mut [0u8; crate::MAX_FRAME_LEN + 8];
        let mut frame = Request::with_slice(buf, Address::Word(0x1234), Write, Crc::on()).unwrap();
        for _ in 0..126 {
            frame.push(&0xABCDu16).unwrap();
        }

        assert_eq!(frame.finalize(), Err(Error::OversizedFrame));
    }

    #[test]
    fn tuple_to_slice_crc() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0E, 0xB4];
        let data = TestTuple::new();

        let mut frame = Request::with_slice(buf, Address::Word(0x00DE), Write, Crc::on()).unwrap();
        frame.push(&data).unwrap();
        let output = frame.finalize().unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn tuple_to_slice_nocrc() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34];
        let data = TestTuple::new();

        let mut frame = Request::with_slice(buf, Address::Word(0x00DE), Write, Crc::off()).unwrap();
        frame.push(&data).unwrap();
        let output = frame.finalize().unwrap();
        assert_eq!(output, expected);
    }

    #[cfg(feature = "heapless")]
    #[test]
    fn tuple_to_hvec() {
        let expected: Vec<u8, 12> =
            Vec::from_slice(&[0x5A, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0E, 0xB4]).unwrap();
        let data = TestTuple::new();

        let mut frame = Request::with_hvec(Address::Word(0x00DE), Write, Crc::on()).unwrap();
        frame.push(&data).unwrap();
        let output: Vec<u8, 12> = frame.finalize().unwrap();
        assert_eq!(output, expected);
    }

    #[cfg(feature = "heapless")]
    #[test]
    fn tuple_to_hvec_nocrc() {
        let expected: Vec<u8, 10> =
            Vec::from_slice(&[0x5A, 0xA5, 7, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34]).unwrap();
        let data = TestTuple::new();

        let mut frame = Request::with_hvec(Address::Word(0x00DE), Write, Crc::off()).unwrap();
        frame.push(&data).unwrap();
        let output: Vec<u8, 10> = frame.finalize().unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn read_word() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 4, 0x83, 0x12, 0x34, 4];

        let output = Request::with_slice(buf, Address::Word(0x1234), Read(4), Crc::off())
            .unwrap()
            .finalize()
            .unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn read_register() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 4, 0x81, 0x01, 0x02, 3];

        let output = Request::with_slice(buf, Address::Register(0x01, 0x02), Read(3), Crc::off())
            .unwrap()
            .finalize()
            .unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn read_dword() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 6, 0x87, 0x00, 0x01, 0x02, 0x03, 2];

        let output = Request::with_slice(buf, Address::Dword(0x0001_0203), Read(2), Crc::off())
            .unwrap()
            .finalize()
            .unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn write_curve() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 4, 0x84, 0x01, 0x12, 0x34];

        let mut frame = Request::with_slice(buf, Address::Curve(0x01), Write, Crc::off()).unwrap();
        frame.push(&0x1234u16).unwrap();
        let output = frame.finalize().unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn write_str() {
        let buf = &mut [0u8; 20];
        let expected = &[0x5A, 0xA5, 5, 0x82, 0x12, 0x34, 0x48, 0x69];

        let mut frame = Request::with_slice(buf, Address::Word(0x1234), Write, Crc::off()).unwrap();
        frame.push("Hi").unwrap();
        let output = frame.finalize().unwrap();
        assert_eq!(output, expected);
    }

    // Compile test: push should not be available on a Read request
    /// ```compile_fail
    /// let buf = &mut [0u8; 20];
    /// let mut frame = Request::with_slice(buf, Address::Word(0), Read(0), Crc::off()).unwrap();
    /// frame.push(&42u8).unwrap(); // push is not available on a Read request
    /// ```
    #[test]
    fn read_request_push_should_not_compile() {}

    /// A checksum that records what it was asked to cover and returns a fixed
    /// value, so the trailer is unmistakable.
    struct StubCrc {
        covered: usize,
    }

    impl Checksum for StubCrc {
        fn compute(&mut self, data: &[u8]) -> u16 {
            self.covered = data.len();
            0xAABB
        }
    }

    #[test]
    fn user_checksum_is_used_and_written_little_endian() {
        let buf = &mut [0u8; 20];
        let mut stub = StubCrc { covered: 0 };

        let mut frame = Request::with_slice(buf, Address::Word(0x1234), Write, Some(&mut stub)).unwrap();
        frame.push(&0x5566u16).unwrap();
        let output = frame.finalize().unwrap();

        // cmd + addr + payload, then the stub's value little-endian.
        assert_eq!(output, &[0x5A, 0xA5, 7, 0x82, 0x12, 0x34, 0x55, 0x66, 0xBB, 0xAA]);
        // The checksum covers everything after the length byte, not the header.
        assert_eq!(stub.covered, 5);
    }
}
