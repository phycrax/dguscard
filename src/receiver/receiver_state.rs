#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReceiverState<const CRC_ENABLED: bool> {
    Initial,
    HeaderHigh,
    HeaderLow,
    Length { length: u8 },
    Command { length: u8 },
    DataStream { length: u8 },
    ChecksumLow { checksum: u8 },
    ChecksumHigh { checksum: u8 },
}

impl<const CRC_ENABLED: bool> ReceiverState<CRC_ENABLED> {
    pub fn next(self, byte: u8) -> Self {
        use ReceiverState::*;
        match self {
            Initial => HeaderHigh,

            HeaderHigh => HeaderLow,

            HeaderLow => Length { length: byte },

            Length { length } => Command { length: length - 1 },

            Command { length } => DataStream { length: length - 1 },

            DataStream { length } => {
                if CRC_ENABLED && (length == 2) {
                    ChecksumLow { checksum: byte }
                } else {
                    DataStream { length: length - 1 }
                }
            }

            ChecksumLow { checksum } => ChecksumHigh { checksum },

            ChecksumHigh { .. } => panic!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ReceiverState::*;

    #[test]
    fn ack() {
        let mut rs = ReceiverState::<true>::Initial;
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        let state = [
            HeaderHigh,
            HeaderLow,
            Length { length: 5 },
            Command { length: 4 },
            DataStream { length: 3 },
            DataStream { length: 2 },
            ChecksumLow { checksum: 0xA5 },
            ChecksumHigh { checksum: 0xA5 },
        ];

        for (i, data) in packet.into_iter().enumerate() {
            rs = rs.next(data);
            assert_eq!(rs, state[i]);
        }
    }

    #[test]
    fn ack_nocrc() {
        let mut rs = ReceiverState::<false>::Initial;
        let packet = [0x5A, 0xA5, 3, 0x82, b'O', b'K'];
        let state: [ReceiverState<false>; 6] = [
            HeaderHigh,
            HeaderLow,
            Length { length: 3 },
            Command { length: 2 },
            DataStream { length: 1 },
            DataStream { length: 0 },
        ];
        for (i, data) in packet.into_iter().enumerate() {
            rs = rs.next(data);
            assert_eq!(rs, state[i]);
        }
    }
}
