pub trait Crc16Modbus {
    fn checksum(&self, bytes: &[u8]) -> u16;
}

#[cfg(feature = "crc")]
pub struct CrcEngine;

#[cfg(feature = "crc")]
impl Crc16Modbus for CrcEngine {
    fn checksum(&self, bytes: &[u8]) -> u16 {
        use crc::{Crc, CRC_16_MODBUS};
        const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
        CRC.checksum(bytes)
    }
}
