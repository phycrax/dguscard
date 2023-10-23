use super::*;

pub struct Packet {
    data: [u8; MAX_DATA],
}

impl Packet {
    //todo: only accept write cmds, return error otherwise
    pub fn new(cmd: Cmd, addr: u16) -> Packet {
        let mut packet = Packet {
            data: [0; MAX_DATA],
        };
        packet.data[0] = HDR0;
        packet.data[1] = HDR1;
        // index 2 is skipped, it tracks len
        packet.append(cmd as u8); //index 3
        packet.append(addr); //index 4 and 5
        packet
    }

    // todo: test dwin response if len is oddnum?
    // todo: how to ensure payload is aligned if there is an odd byte?
    // Note: Consumes the package, package invalid afterwards
    pub fn consume(mut self) -> (usize, [u8; MAX_DATA]) {
        if CRC_ENABLED {
            // calculate crc from [CMD] to end.
            let len = self.data[2] as usize + 3;
            let data_bytes = &self.data[3..len];
            let crc = CRC.compute(data_bytes).to_le_bytes();
            // CRC should be little endian in payload, so can't use add_u16
            self.append(crc[0]);
            self.append(crc[1]);
        }

        // actual payload len is len + 3
        // +2 header and +1 len itself
        // todo: 3 scattered everywhere RN, consider making it const
        let len = self.data[2] as usize + 3;
        (len, self.data)
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
                    self.data[self.data[2] as usize + 3] = byte;
                    self.data[2] += 1;
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
        let mut packet = Packet::new(Cmd::Write16, 0x00DE);
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
