use crate::error::{Error, Result};
use serde::Serialize;
use serializer::Serializer;

pub(crate) mod serializer;

#[derive(Copy, Clone)]
pub struct Config {
    pub header: u16,
    pub crc: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            header: 0x5AA5,
            crc: true,
        }
    }
}

pub trait DwinVariable {
    const ADDRESS: u16;
}

pub fn to_slice<'b, T>(value: &T, buf: &'b mut [u8], config: Config) -> Result<&'b [u8]>
where
    T: Serialize + DwinVariable,
{
    let mut serializer = Serializer::new(buf, config.header, T::ADDRESS);
    value.serialize(&mut serializer)?;
    serializer.finalize(config.crc)
}
