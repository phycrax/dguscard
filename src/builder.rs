use crate::{Config, Crc16Modbus, FrameCommand};

pub struct FrameBuilder<'a, T> {
    config: Config<T>,
    data: &'a mut [u8],
    index: usize,
}

impl<'a, T: Crc16Modbus> FrameBuilder<'a, T> {
    pub fn new(
        buffer: &'a mut [u8],
        config: Config<T>,
        command: FrameCommand,
        address: u16,
    ) -> Self {
        assert!(buffer.len() >= 8, "Buffer too small");
        assert!(buffer.len() < u8::MAX as usize, "Buffer too large");
        let header = config.header;
        let mut frame = Self {
            config,
            data: buffer,
            index: 0,
        };
        frame.append(header); // -> [HEADER:2]
        frame.append(0u8); // -> [LEN:1]
        frame.append(command as u8); // -> [CMD:1]
        frame.append(address); // -> [ADDR:2]
        frame
    }

    // todo: test dwin response if len is oddnum?
    // todo: how to ensure payload is aligned if there is an odd byte?
    // ToDo: any way to prevent using other methods after calling this? Maybe state pattern?
    pub fn consume(mut self) -> &'a [u8] {
        if self.config.crc {
            // calculate crc from [CMD] to end.
            let crc = self
                .config
                .crc_engine
                .checksum(&self.data[3..self.index])
                .to_le_bytes();
            // CRC should be little endian in payload, so can't use add_u16
            self.append(crc[0]);
            self.append(crc[1]);
        }
        self.data[2] = (self.index - 3) as u8; //[LEN:1] -> first 3 bytes are excluded
        &self.data[..self.index]
    }
}

impl<'a, T: Crc16Modbus> FrameBuilder<'a, T> {
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
        impl<'a, T> Append<$ty> for FrameBuilder<'a, T> {
            fn append(&mut self, data: $ty) {
                let bytes = data.to_be_bytes();
                for byte in bytes {
                    self.data[self.index] = byte;
                    self.index += 1;
                }
            }
        }
    )+)
}

impl_append! { u8 i8 u16 i16 u32 i32 u64 i64 f32 f64 }

pub fn change_page(buffer: &mut [u8], page: u16) -> &[u8] {
    let mut builder = FrameBuilder::new(buffer, Default::default(), FrameCommand::Write16, 0x0084);
    builder.append_u16(0x5A01);
    builder.append_u16(page);
    builder.consume()
}

pub fn set_brightness(
    buffer: &mut [u8],
    standby_level: u8,
    sleep_level: u8,
    secs_to_sleep: u16,
) -> &[u8] {
    // todo assert levels?
    let mut builder = FrameBuilder::new(buffer, Default::default(), FrameCommand::Write16, 0x0082);
    builder.append_u8(standby_level);
    builder.append_u8(sleep_level);
    builder.append_u16(secs_to_sleep);
    builder.consume()
}

pub fn set_bg_icl(buffer: &mut [u8], icl: u16) -> &[u8] {
    let mut builder = FrameBuilder::new(buffer, Default::default(), FrameCommand::Write16, 0x00DE);
    builder.append_u16(0x5A00);
    builder.append_u16(icl);
    builder.consume()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_background_icl_output() {
        let mut buffer = [0u8; 50];
        let mut packet = FrameBuilder::new(
            &mut buffer,
            Default::default(),
            FrameCommand::Write16,
            0x00DE,
        );

        packet.append_u16(0x5A00);
        packet.append_u16(0x1234);
        let bytes = packet.consume();

        if bytes.len() != 12 {
            panic!("Len should have been 12");
        }

        let test_output = [
            0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
        ];

        assert_eq!(bytes, &test_output);
    }
}
