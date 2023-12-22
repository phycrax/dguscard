use super::*;

#[derive(Debug)]
pub enum ParseError {
    BadHdr0,
    BadHdr1,
    BadLen,
    Overrun,
    BadCrc,
    BadCmd,
}

pub enum ParseResult {
    Ack,
    Data { cmd: Cmd, addr: u16, wlen: u8 },
}

enum ParserState {
    WaitingHeader,
    ReceivingHeader,
    WaitingLength,
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
            state: ParserState::WaitingHeader,
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

    const fn check_header_first_byte(byte_in: u8) -> Result<(), ParseError> {
        if byte_in == HEADER.to_be_bytes()[0] {
            Ok(())
        } else {
            Err(ParseError::BadHdr0)
        }
    }

    const fn check_header_second_byte(byte_in: u8) -> Result<(), ParseError> {
        if byte_in == HEADER.to_be_bytes()[1] {
            Ok(())
        } else {
            Err(ParseError::BadHdr1)
        }
    }

    const fn check_length(byte_in: u8) -> Result<(), ParseError> {
        let min_len = if CRC_ENABLED { 5 } else { 3 };
        let max_len = BUFFER_SIZE as u8;
        if byte_in >= min_len && byte_in <= max_len {
            Ok(())
        } else {
            Err(ParseError::BadLen)
        }
    }

    fn check_crc16(bytes: &[u8]) -> Result<(), ParseError> {
        let len = bytes.len();
        let recv_crc = u16::from_le_bytes([bytes[len - 2], bytes[len - 1]]);
        if recv_crc == CRC.checksum(&bytes[..len - 2]) {
            Ok(())
        } else {
            Err(ParseError::BadCrc)
        }
    }

    const fn is_ack(len: usize, addr: u16) -> bool {
        let is_ack_len = if CRC_ENABLED { len == 5 } else { len == 3 };
        is_ack_len && addr == u16::from_le_bytes([b'O', b'K'])
    }

    fn parse(bytes: &[u8]) -> Result<Option<ParseResult>, ParseError> {
        let cmd: Cmd = bytes[0].into();
        if cmd == Cmd::Undefined {
            return Err(ParseError::BadCmd);
        }

        let addr = u16::from_be_bytes([bytes[1], bytes[2]]);

        if Self::is_ack(bytes.len(), addr) {
            return Ok(Some(ParseResult::Ack));
        }

        Ok(Some(ParseResult::Data {
            wlen: bytes[3],
            addr,
            cmd,
        }))
    }

    // Async fn possible?
    pub fn consume(&mut self, byte: u8) -> Result<Option<ParseResult>, ParseError> {
        use ParseError::*;
        use ParserState::*;
        match self.state {
            WaitingHeader => {
                unsafe {
                    self.buffer.set_len(0);
                }
                Self::check_header_first_byte(byte)?;
                self.state = ReceivingHeader;
            }
            ReceivingHeader => {
                Self::check_header_second_byte(byte)?;
                self.state = WaitingLength;
            }
            WaitingLength => {
                Self::check_length(byte)?;
                self.state = ReceivingData { remaining: byte };
            }
            ReceivingData { ref mut remaining } => {
                self.buffer.push(byte).map_err(|_| Overrun)?;
                *remaining -= 1;
                if *remaining == 0 {
                    if CRC_ENABLED {
                        Self::check_crc16(&self.buffer)?;
                        return Self::parse(&self.buffer[..self.buffer.len() - 2]);
                    } else {
                        return Self::parse(&self.buffer);
                    }
                }
            }
        }
        Ok(None)
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
        let mut parser = Parser::<0x5AA5, 64, true>::new();
        let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
        for i in packet {
            if let Some(result) = parser.consume(i).unwrap() {
                if let ParseResult::Data { addr, .. } = result {
                    if addr == 0xAABB {
                        // success
                        return;
                    }
                } else {
                    panic!("Expected Data");
                }
            }
        }
        // Shouldn't reach here
        panic!("Wrong adress");
    }
}
