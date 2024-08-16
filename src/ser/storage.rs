use crate::error::{Error, Result};
use core::ops::{Deref, DerefMut};

pub trait Storage: Deref<Target = [u8]> + DerefMut<Target = [u8]> {
    type Output;
    fn try_push(&mut self, data: u8) -> Result<()>;
    fn finalize(self) -> Self::Output;
}

pub struct Slice<'a> {
    buf: &'a mut [u8],
    index: usize,
}

impl<'a> Slice<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, index: 0 }
    }
}

impl<'a> Storage for Slice<'a> {
    type Output = &'a mut [u8];

    #[inline(always)]
    fn try_push(&mut self, b: u8) -> Result<()> {
        *self
            .buf
            .get_mut(self.index)
            .ok_or(Error::SerializeBufferFull)? = b;
        self.index += 1;
        Ok(())
    }

    // Will panic with len() < 3
    fn finalize(self) -> Self::Output {
        &mut self.buf[..self.index]
    }
}

impl<'a> Deref for Slice<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buf[..self.index]
    }
}

impl<'a> DerefMut for Slice<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf[..self.index]
    }
}

#[cfg(feature = "heapless")]
impl<const N: usize> Storage for heapless::Vec<u8, N> {
    type Output = heapless::Vec<u8, N>;

    #[inline(always)]
    fn try_push(&mut self, data: u8) -> Result<()> {
        self.push(data).map_err(|_| Error::SerializeBufferFull)
    }

    // Will panic with len() < 3
    fn finalize(self) -> Self::Output {
        self
    }
}
