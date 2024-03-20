use super::*;

pub struct Packet {
    config: &'static Config,
    data: Vec<u8, PACKET_MAX_SIZE>,
}

impl Packet {
    pub fn new(config: &'static Config, cmd: Cmd, addr: u16) -> Packet {
        let mut packet = Packet {
            config,
            data: Vec::new(),
        };
        packet.append(packet.config.header1);
        packet.append(packet.config.header2);
        packet.append(0u8); // ->[LEN]
        packet.append(cmd as u8); //index 3 is CMD
        packet.append(addr); //index 4 and 5 is ADDR
        packet
    }

    // todo: test dwin response if len is oddnum?
    // todo: how to ensure payload is aligned if there is an odd byte?
    // ToDo: any way to prevent using other methods after calling this? Maybe state pattern?
    fn finalize(&mut self) {
        if self.config.crc_enabled {
            // calculate crc from [CMD] to end.
            let crc = CRC.checksum(&self.data[3..]).to_le_bytes();
            // CRC should be little endian in payload, so can't use add_u16
            self.append(crc[0]);
            self.append(crc[1]);
        }
        self.data[2] = (self.data.len() - 3) as u8; //[LEN:1] -> first 3 bytes are excluded
    }

    // Note: Consumes the package, package invalid afterwards
    pub fn consume(mut self) -> (usize, [u8; PACKET_MAX_SIZE]) {
        self.finalize();
        let mut array = [0; PACKET_MAX_SIZE];
        let len = self.data.len();
        array[..len].clone_from_slice(self.data.as_slice());
        (len, array)
    }

    // compared to consume, this would be more performant
    pub fn as_slice(&mut self) -> &[u8] {
        self.finalize();
        &self.data
    }
}

pub trait Append<T> {
    fn append(&mut self, data: T);
}

// Macro for blanket implementation of appending primitive types to the payload
// Cons: numeric literals must be type annotated.
// Note: Manually implement these if it becomes a pain. See unit test.
macro_rules! impl_append {
    ($($ty:ident)+) => ($(
        impl Append<$ty> for Packet {
            fn append(&mut self, data: $ty) {
                let bytes = data.to_be_bytes();
                for byte in bytes {
                    self.data.push(byte);
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
        let mut packet = Packet::new(
            &Config {
                header1: 0x5A,
                header2: 0xA5,
                crc_enabled: true,
            },
            Cmd::Write16,
            0x00DE,
        );

        // Example of the pain with number literals, annotation needed.
        packet.append(0x5A00_u16);
        packet.append(0x1234_u16);
        let (len, data) = packet.consume();

        if len != 12 {
            panic!("Len should have been 12");
        }

        let test_output = [
            0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
        ];

        for i in 0..12 {
            assert!(
                test_output[i] == data[i],
                "Expected:{} Received:{} At Index:{}",
                test_output[i],
                data[i],
                i
            );
        }
    }
}
