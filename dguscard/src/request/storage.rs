//! # Serialization outputs
//!
//! i.e., the output medium of the serialization, e.g. whether the data is serialized to a [`u8`] slice, or a [`Vec<u8, N>`][heapless::Vec].

use core::ops::{Deref, DerefMut};

#[cfg(feature = "heapless")]
pub use hvec::*;

use crate::MAX_FRAME_LEN;
use crate::request::Error;

/// Serialization storage, the output medium of the serialization
///
/// Serialization buffers can implement this trait to be used as an output with the serializer
/// e.g. whether the data is serialized to a [`u8`] slice, or a [`Vec<u8, N>`][heapless::Vec].
pub trait Storage: Deref<Target = [u8]> + DerefMut<Target = [u8]> {
    /// What this storage "resolves" to when the serialization is complete,
    /// such as a [`u8`] slice, or a [`Vec<u8, N>`][heapless::Vec].
    type Output;

    /// Can be implemented when there is a more efficient way of processing
    /// multiple bytes at once, such as copying a slice to the output, rather than iterating over one byte
    /// at a time.
    #[inline]
    fn try_extend(&mut self, data: &[u8]) -> Result<(), Error> {
        data.iter().try_for_each(|d| self.try_push(*d))
    }

    /// Can be used to push a single byte to be modified and/or stored.
    ///
    /// Implementors must refuse to grow beyond 258, returning [`Error::BufferFull`] instead.
    fn try_push(&mut self, data: u8) -> Result<(), Error>;

    /// Finalize the serialization process.
    fn finalize(self) -> Self::Output;
}

/// A storage type that uses plain [`u8`] slice
///
/// Stores the serialized bytes into a plain [`u8`] slice.
/// Resolves into a sub-slice of the given slice buffer.
#[derive(Debug)]
pub struct Slice<'a> {
    buf: &'a mut [u8],
    index: usize,
}

impl<'a> Slice<'a> {
    /// Create a new `Slice` from given backing buffer
    ///
    /// # Panics
    ///
    /// Panics if `buf` is longer than 258.
    pub const fn new(buf: &'a mut [u8]) -> Self {
        assert!(buf.len() <= MAX_FRAME_LEN, "Request buffer should be <= 258 bytes");
        Self { buf, index: 0 }
    }
}

impl<'a> Storage for Slice<'a> {
    type Output = &'a mut [u8];

    #[inline]
    fn try_push(&mut self, b: u8) -> Result<(), Error> {
        *self.buf.get_mut(self.index).ok_or(Error::BufferFull)? = b;
        self.index += 1;
        Ok(())
    }

    fn finalize(self) -> Self::Output {
        &mut self.buf[..self.index]
    }
}

impl Deref for Slice<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buf[..self.index]
    }
}

impl DerefMut for Slice<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf[..self.index]
    }
}

#[cfg(feature = "heapless")]
mod hvec {
    use heapless::Vec;

    use super::*;

    /// A storage type that wraps [`Vec<u8, N>`][heapless::Vec]
    ///
    /// Stores the serialized bytes and resolves into a [`Vec<u8, N>`][heapless::Vec].
    /// This is a stack allocated data structure, with a fixed maximum size and variable amount of contents.
    #[derive(Debug)]
    pub struct HVec<const N: usize>(Vec<u8, N>);

    impl<const N: usize> HVec<N> {
        /// Create a new `HVec` to be used for storing serialized data.
        ///
        /// `N` may not exceed 258
        pub const fn new() -> Self {
            const {
                assert!(
                    N <= MAX_FRAME_LEN,
                    "Request buffer should be <= 258 bytes, the largest describable DGUS frame"
                );
            }
            Self(Vec::new())
        }
    }

    impl<const N: usize> Default for HVec<N> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<const N: usize> Storage for HVec<N> {
        type Output = Vec<u8, N>;

        #[inline]
        fn try_extend(&mut self, data: &[u8]) -> Result<(), Error> {
            self.0.extend_from_slice(data).map_err(|_| Error::BufferFull)
        }

        #[inline]
        fn try_push(&mut self, data: u8) -> Result<(), Error> {
            self.0.push(data).map_err(|_| Error::BufferFull)
        }

        fn finalize(self) -> Self::Output {
            self.0
        }
    }

    impl<const N: usize> Deref for HVec<N> {
        type Target = [u8];

        fn deref(&self) -> &Self::Target {
            self.0.deref()
        }
    }

    impl<const N: usize> DerefMut for HVec<N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.0.deref_mut()
        }
    }
}
