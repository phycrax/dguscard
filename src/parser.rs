use super::*;
use core::mem;

#[derive(Debug)]
pub enum ParseErr {
    BadHdr0,
    BadHdr1,
    BadLen,
    Overrun,
    BadCrc,
    BadCmd,
}

//todo: refactor this, parse should return a struct with arrayvec as data
//todo: widget may decide what to do with the data
pub enum ParseOk {
    Ack,
    Data8 {
        addr: u16,
        wlen: usize,
        data: [u8; MAX_DATA / mem::size_of::<u8>()],
    },
    Data16 {
        addr: u16,
        wlen: usize,
        data: [u16; MAX_DATA / mem::size_of::<u16>()],
    },
    Data32 {
        addr: u16,
        wlen: usize,
        data: [u32; MAX_DATA / mem::size_of::<u32>()],
    },
}

enum ParserState {
    ReceivedFirstHeaderByte,
    ReceivedSecondHeaderByte,
    ReceivedLength,
    ReceivingData { remaining: u8 },
}

pub struct Parser<const H: u16, const N: usize, const C: bool> {
    state: ParserState,
    data: Vec<u8, N>,
}

impl<const HEADER: u16, const BUFFER_SIZE: usize, const CRC_ENABLED: bool> Default
    for Parser<HEADER, BUFFER_SIZE, CRC_ENABLED>
{
    fn default() -> Self {
        assert!(BUFFER_SIZE < 246, "<N> should be 200 or less");
        Self {
            state: ParserState::ReceivedFirstHeaderByte,
            data: Vec::new(),
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
        byte_in < BUFFER_SIZE as u8
    }

    fn check_crc16(bytes: &[u8]) -> bool {
        let len = bytes.len();
        let recv_crc = u16::from_le_bytes([bytes[len - 2], bytes[len - 1]]);
        recv_crc != CRC.checksum(&bytes[..len - 2])
    }

    fn parse(bytes: &[u8]) -> Result<ParseOk, ParseErr> {
        // Is it ack?
        if bytes.len() == 3 + (CRC_ENABLED as usize * 2)
            && ((bytes[0] == Cmd::WriteRegister as u8)
                || (bytes[0] == Cmd::Write16 as u8)
                || (bytes[0] == Cmd::Write32 as u8))
            && bytes[1] == b'O'
            && bytes[2] == b'K'
        {
            return Ok(ParseOk::Ack);
        }

        // Lazy conversion
        let cmd: Cmd = unsafe { mem::transmute(bytes[0]) };
        let addr = u16::from_be_bytes([bytes[1], bytes[2]]);
        let wlen = bytes[3] as usize;
        let data_bytes = &bytes[4..];

        match cmd {
            Cmd::ReadRegister => {
                let data = data_bytes.try_into().unwrap();
                Ok(ParseOk::Data8 { addr, wlen, data })
            }
            Cmd::Read16 => {
                let mut data = [0u16; MAX_DATA / mem::size_of::<u16>()];
                for (i, bytes) in data_bytes.chunks(mem::size_of::<u16>()).enumerate() {
                    data[i] = u16::from_be_bytes(bytes.try_into().unwrap());
                }
                Ok(ParseOk::Data16 { addr, wlen, data })
            }
            Cmd::Read32 => {
                let mut data = [0u32; MAX_DATA / mem::size_of::<u32>()];
                for (i, bytes) in data_bytes.chunks(mem::size_of::<u32>()).enumerate() {
                    data[i] = u32::from_be_bytes(bytes.try_into().unwrap());
                }
                Ok(ParseOk::Data32 { addr, wlen, data })
            }
            _ => Err(ParseErr::BadCmd),
        }
    }

    // Async fn possible?
    pub fn decode(&mut self, byte_in: u8) -> Option<Result<ParseOk, ParseErr>> {
        use ParseErr::*;
        use ParserState::*;
        match self.state {
            ReceivedFirstHeaderByte => {
                self.data.clear();
                if Self::check_header_first_byte(byte_in) {
                    self.state = ReceivedSecondHeaderByte;
                    None
                } else {
                    self.state = ReceivedFirstHeaderByte;
                    Some(Err(BadHdr0))
                }
            }
            ReceivedSecondHeaderByte => {
                if Self::check_header_second_byte(byte_in) {
                    self.state = ReceivedLength;
                    None
                } else {
                    self.state = ReceivedFirstHeaderByte;
                    Some(Err(BadHdr1))
                }
            }
            ReceivedLength => {
                if Self::check_length(byte_in) {
                    self.state = ReceivingData { remaining: byte_in };
                    None
                } else {
                    self.state = ReceivedFirstHeaderByte;
                    Some(Err(BadLen))
                }
            }
            ReceivingData { mut remaining } => {
                remaining -= 1;
                if self.data.push(byte_in).is_err() {
                    Some(Err(Overrun))
                } else if remaining == 0 {
                    if CRC_ENABLED {
                        if Self::check_crc16(&self.data) {
                            Some(Self::parse(&self.data[..self.data.len() - 2]))
                        } else {
                            Some(Err(BadCrc))
                        }
                    } else {
                        Some(Self::parse(&self.data))
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
            if let Some(result) = parser.decode(i) {
                if let ParseOk::Data16 { addr, .. } = result.unwrap() {
                    if addr != 0xAABB {
                        panic!("Wrong adress");
                    }
                } else {
                    panic!("Expected Data16");
                }
            }
        }
    }
}
