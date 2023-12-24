#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReceiverState {
    Initial,
    HeaderHigh,
    HeaderLow,
    Length { length: u8 },
    Command { length: u8 },
    DataStream { length: u8 },
}

impl ReceiverState {
    pub fn next(self, byte: u8) -> Self {
        use ReceiverState::*;
        match self {
            Initial => HeaderHigh,
            HeaderHigh => HeaderLow,
            HeaderLow => Length { length: byte },
            Length { length } => Command { length: length - 1 },
            Command { length } => DataStream { length: length - 1 },
            DataStream { length: 0 } => panic!("Unexpected state, rearm?"),
            DataStream { length } => DataStream { length: length - 1 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ReceiverState::*;

    #[test]
    fn ack() {
        let mut rs = ReceiverState::Initial;
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        let state = [
            HeaderHigh,
            HeaderLow,
            Length { length: 5 },
            Command { length: 4 },
            DataStream { length: 3 },
            DataStream { length: 2 },
            DataStream { length: 1 },
            DataStream { length: 0 },
        ];

        for (i, data) in packet.into_iter().enumerate() {
            rs = rs.next(data);
            assert_eq!(rs, state[i]);
        }
    }

    #[test]
    #[should_panic(expected = "Unexpected state, rearm?")]
    fn len_exceeded() {
        let mut rs = ReceiverState::Initial;
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0x00];
        let state = [
            HeaderHigh,
            HeaderLow,
            Length { length: 5 },
            Command { length: 4 },
            DataStream { length: 3 },
            DataStream { length: 2 },
            DataStream { length: 1 },
            DataStream { length: 0 },
            DataStream { length: 0 },
        ];

        for (i, data) in packet.into_iter().enumerate() {
            rs = rs.next(data);
            assert_eq!(rs, state[i]);
        }
    }
}
