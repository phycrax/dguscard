use crate::error::{Error, Result};

pub(crate) trait Output {
    type Out;

    fn as_bytes(&self) -> &[u8];

    fn try_push(&mut self, data: u8) -> Result<()>;

    fn finalize(self) -> Self::Out;
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

impl<'a> Output for Slice<'a> {
    type Out = &'a mut [u8];

    fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.index]
    }

    #[inline(always)]
    fn try_push(&mut self, b: u8) -> Result<()> {
        *self
            .buf
            .get_mut(self.index)
            .ok_or(Error::SerializeBufferFull)? = b;
        self.index += 1;
        Ok(())
    }

    fn finalize(self) -> Self::Out {
        self.buf[2] = (self.index - 3) as u8;
        &mut self.buf[..self.index]
    }
}

#[cfg(feature = "heapless")]
impl<const N: usize> Output for heapless::Vec<u8, N> {
    type Out = heapless::Vec<u8, N>;

    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }

    #[inline(always)]
    fn try_push(&mut self, data: u8) -> Result<()> {
        self.push(data).map_err(|_| Error::SerializeBufferFull)
    }

    fn finalize(mut self) -> Self::Out {
        self[2] = (self.len() - 3) as u8;
        self
    }
}
