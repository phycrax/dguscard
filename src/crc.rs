use crate::{builder::FrameBuilder, parser::FrameParser};

pub trait Crc16Modbus {
    fn checksum(bytes: &[u8]) -> u16;
}

#[cfg(feature = "crc")]
impl Crc16Modbus for FrameParser {
    fn checksum(bytes: &[u8]) -> u16 {
        use crc::{Crc, CRC_16_MODBUS};
        const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
        CRC.checksum(bytes)
    }
}

#[cfg(feature = "crc")]
impl<const N: usize> Crc16Modbus for FrameBuilder<N> {
    fn checksum(bytes: &[u8]) -> u16 {
        use crc::{Crc, CRC_16_MODBUS};
        const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
        CRC.checksum(bytes)
    }
}
