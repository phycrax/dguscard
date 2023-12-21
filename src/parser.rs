use super::*;

pub enum ParseResult<'a> {
    Ack,
    Data {
        data: &'a [u8],
        addr: u16,
        cmd: Cmd,
        wlen: u8,
    },
    BadHdr0,
    BadHdr1,
    BadLen,
    Overrun,
    BadCrc,
    BadCmd,
}

enum ParserState {
    ReceivedFirstHeaderByte,
    ReceivedSecondHeaderByte,
    ReceivedLength,
    ReceivingData { remaining: u8 },
}

pub struct Parser<const H: u16, const N: usize, const C: bool> {
    state: ParserState,
    buffer: Vec<u8, N>,
}

impl<const HEADER: u16, const BUFFER_SIZE: usize, const CRC_ENABLED: bool> Default
    for Parser<HEADER, BUFFER_SIZE, CRC_ENABLED>
{
    fn default() -> Self {
        assert!(BUFFER_SIZE < 246, "<N> should be 200 or less");
        Self {
            state: ParserState::ReceivedFirstHeaderByte,
            buffer: Vec::new(),
        }
    }
}

impl<const HEADER: u16, const BUFFER_SIZE: usize, const CRC_ENABLED: bool>
    Parser<HEADER, BUFFER_SIZE, CRC_ENABLED>
{
    pub fn new() -> Self {
        Default::default()
    }

    const fn check_header_first_byte(byte_in: u8) -> bool {
        byte_in == (HEADER.to_be_bytes()[0])
    }

    const fn check_header_second_byte(byte_in: u8) -> bool {
        byte_in == (HEADER.to_be_bytes()[1])
    }

    const fn check_length(byte_in: u8) -> bool {
        let max_len = BUFFER_SIZE as u8;
        byte_in > 2 && byte_in < max_len
    }

    fn check_crc16(bytes: &[u8]) -> bool {
        let len = bytes.len();
        let recv_crc = u16::from_le_bytes([bytes[len - 2], bytes[len - 1]]);
        recv_crc != CRC.checksum(&bytes[..len - 2])
    }

    const fn check_ack(bytes: &[u8]) -> bool {
        let is_addr_ok = bytes[1] == b'O' && bytes[2] == b'K';
        if CRC_ENABLED {
            is_addr_ok && bytes.len() == 5
        } else {
            is_addr_ok && bytes.len() == 3
        }
    }

    fn parse(bytes: &[u8]) -> ParseResult {
        use ParseResult::*;
        let cmd: Cmd = bytes[0].into();
        if cmd == Cmd::Undefined {
            BadCmd
        }
        // Is it ack?
        else if Self::check_ack(bytes) {
            Ack
        } else {
            Data {
                data: &bytes[4..],
                wlen: bytes[3],
                addr: u16::from_be_bytes([bytes[1], bytes[2]]),
                cmd,
            }
        }
    }

    // Async fn possible?
    pub fn consume(&mut self, byte: u8) -> Option<ParseResult> {
        use ParseResult::*;
        use ParserState::*;
        match self.state {
            ReceivedFirstHeaderByte => {
                unsafe {
                    self.buffer.set_len(0);
                }
                if Self::check_header_first_byte(byte) {
                    self.state = ReceivedSecondHeaderByte;
                    None
                } else {
                    self.state = ReceivedFirstHeaderByte;
                    Some(BadHdr0)
                }
            }
            ReceivedSecondHeaderByte => {
                if Self::check_header_second_byte(byte) {
                    self.state = ReceivedLength;
                    None
                } else {
                    self.state = ReceivedFirstHeaderByte;
                    Some(BadHdr1)
                }
            }
            ReceivedLength => {
                if Self::check_length(byte) {
                    self.state = ReceivingData { remaining: byte };
                    None
                } else {
                    self.state = ReceivedFirstHeaderByte;
                    Some(BadLen)
                }
            }
            ReceivingData { mut remaining } => {
                remaining -= 1;
                if self.buffer.push(byte).is_err() {
                    Some(Overrun)
                } else if remaining == 0 {
                    if CRC_ENABLED {
                        if Self::check_crc16(&self.buffer) {
                            Some(Self::parse(&self.buffer[..self.buffer.len() - 2]))
                        } else {
                            Some(BadCrc)
                        }
                    } else {
                        Some(Self::parse(&self.buffer))
                    }
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //#[test]
    // fn ack_with_crc() {
    //     let parser = Parser::<0x5AA5, 240, true>::new();
    //     let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
    //     parser.parse(&packet).expect("Bad parse!");
    // }
    #[test]
    fn parse_one_u16() {
        let mut parser = Parser::<0x5AA5, 240, true>::new();
        let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
        for i in packet {
            if let Some(result) = parser.consume(i) {
                if let ParseResult::Data { addr, .. } = result {
                    if addr != 0xAABB {
                        panic!("Wrong adress");
                    }
                } else {
                    panic!("Expected Data");
                }
            }
        }
    }
}
