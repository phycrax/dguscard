use crate::{Config, Crc16Modbus, FrameCommand};
use heapless::Vec;

pub struct FrameBuilder<T, const N: usize> {
    config: Config<T>,
    data: Vec<u8, N>,
}

impl<T: Crc16Modbus, const N: usize> FrameBuilder<T, N> {
    const MIN_SIZE: () = { assert!(N >= 8, "Size too small") };
    const MAX_SIZE: () = { assert!(N < u8::MAX as usize, "Size too large") };

    pub fn new(config: Config<T>, command: FrameCommand, address: u16) -> Self {
        // Sanity check
        #[allow(clippy::let_unit_value)]
        {
            let _ = Self::MIN_SIZE;
            let _ = Self::MAX_SIZE;
        }
        let header = config.header;
        let mut packet = Self {
            config,
            data: Vec::new(),
        };
        packet.append(header); // -> [HEADER:2]
        packet.append(0u8); // -> [LEN:1]
        packet.append(command as u8); // -> [CMD:1]
        packet.append(address); // -> [ADDR:2]
        packet
    }

    // todo: test dwin response if len is oddnum?
    // todo: how to ensure payload is aligned if there is an odd byte?
    // ToDo: any way to prevent using other methods after calling this? Maybe state pattern?
    pub fn get(&mut self) -> &[u8] {
        if self.config.crc {
            // calculate crc from [CMD] to end.
            let crc = self
                .config
                .crc_engine
                .checksum(&self.data[3..])
                .to_le_bytes();
            // CRC should be little endian in payload, so can't use add_u16
            self.append(crc[0]);
            self.append(crc[1]);
        }
        self.data[2] = (self.data.len() - 3) as u8; //[LEN:1] -> first 3 bytes are excluded
        &self.data
    }
}

impl<T: Crc16Modbus, const N: usize> FrameBuilder<T, N> {
    pub fn append_u8(&mut self, data: u8) {
        self.append(data);
    }

    pub fn append_u16(&mut self, data: u16) {
        self.append(data);
    }

    pub fn append_u32(&mut self, data: u16) {
        self.append(data);
    }

    pub fn append_u64(&mut self, data: u64) {
        self.append(data);
    }

    pub fn append_i8(&mut self, data: i8) {
        self.append(data);
    }

    pub fn append_i16(&mut self, data: i16) {
        self.append(data);
    }

    pub fn append_i32(&mut self, data: i32) {
        self.append(data);
    }

    pub fn append_i64(&mut self, data: i64) {
        self.append(data);
    }

    pub fn append_f32(&mut self, data: f32) {
        self.append(data);
    }

    pub fn append_f64(&mut self, data: f64) {
        self.append(data);
    }
}

trait Append<T> {
    fn append(&mut self, data: T);
}

// Macro for blanket implementation of appending primitive types to the payload
macro_rules! impl_append {
    ($($ty:ident)+) => ($(
        impl<T, const N: usize> Append<$ty> for FrameBuilder<T, N> {
            fn append(&mut self, data: $ty) {
                let bytes = data.to_be_bytes();
                for byte in bytes {
                    let _ = self.data.push(byte);
                }
            }
        }
    )+)
}

impl_append! { u8 i8 u16 i16 u32 i32 u64 i64 f32 f64 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_background_icl_output() {
        let mut packet =
            FrameBuilder::<_, 50>::new(Default::default(), FrameCommand::Write16, 0x00DE);

        packet.append_u16(0x5A00);
        packet.append_u16(0x1234);
        let bytes = packet.get();

        if bytes.len() != 12 {
            panic!("Len should have been 12");
        }

        let test_output = [
            0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
        ];

        assert_eq!(bytes, &test_output);
    }
}
