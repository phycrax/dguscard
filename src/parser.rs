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
    Initial,
    FirstHeader,
    SecondHeader,
    Data { remaining: u8 },
}

pub struct Parser<const H: u16, const N: usize, const C: bool> {
    state: ParserState,
    data: Vec<u8, N>,
}

impl<const H: u16, const N: usize, const C: bool> Default for Parser<H, N, C> {
    fn default() -> Self {
        assert!(N < 246 as usize, "<N> should be 200 or less");
        Self {
            state: ParserState::Initial,
            data: Vec::new(),
        }
    }
}

impl<const H: u16, const N: usize, const C: bool> Parser<H, N, C> {
    const fn check_header_first_byte(byte_in: u8) -> bool {
        byte_in == (H.to_be_bytes()[0])
    }

    const fn check_header_second_byte(byte_in: u8) -> bool {
        byte_in == (H.to_be_bytes()[1])
    }

    const fn check_length(byte_in: u8) -> bool {
        byte_in < N as u8
    }

    pub fn new() -> Self {
        Default::default()
    }

    pub fn decode(&mut self, byte_in: u8) -> Option<Result<ParseOk, ParseErr>> {
        use ParserState::*;
        match self.state {
            Initial => {
                self.data.clear();
                if Self::check_header_first_byte(byte_in) {
                    self.state = FirstHeader;
                } else {
                    self.state = Initial;
                    return Some(Err(ParseErr::BadHdr0));
                }
            }
            FirstHeader => {
                if Self::check_header_second_byte(byte_in) {
                    self.state = SecondHeader;
                } else {
                    //error handling
                    self.state = Initial;
                    return Some(Err(ParseErr::BadHdr1));
                }
            }
            SecondHeader => {
                if Self::check_length(byte_in) {
                    //todo set const literal
                    self.state = Data { remaining: byte_in };
                } else {
                    //error handling
                    self.state = Initial;
                    return Some(Err(ParseErr::BadLen));
                }
            }
            Data { mut remaining } => {
                remaining -= 1;
                if self.data.push(byte_in).is_err() {
                    return Some(Err(ParseErr::Overrun));
                }
                if remaining == 0 {
                    return Some(self.parse(&self.data));
                }
            }
            _ => panic!(),
        }
        None
    }

    // Protocol: [HDR:2][LEN:1][CMD:1][ADDR:2][WLEN:1][DATA:N][CRC:2]
    // HDR: Header frames
    // LEN: Size of the packet starting from CMD, includes CRC
    // CMD: Refer to DGUS DevGuide
    // ADDR: Address of the DWIN variable
    // CRC: is optional, uses CRC_16_MODBUS, little endian
    // DATA: Max 246 bytes. Each DWIN address holds 2 bytes, big endian
    // WLEN: byte, word or dword length based on command
    // Exceptions: Write commands return ACK.
    // ACK: [HDR:2][LEN:1][CMD:1]['O''K':2][CRC:2]
    pub fn parse(&self, bytes: &[u8]) -> Result<ParseOk, ParseErr> {
        let len = bytes.len();

        // Calculate CRC16 if enabled
        if C {
            // Last 2 bytes are incoming CRC
            let recv_crc = u16::from_le_bytes([bytes[len - 2], bytes[len - 1]]);
            // Pass the slice without CRC
            check_crc16(&bytes[..(len - 2)], recv_crc)?;
        }

        let data_bytes = &bytes[..(len - (C as usize * 2))];

        // Is it ack?
        if len == 3 + (N as usize * 2)
            && ((data_bytes[0] == Cmd::WriteRegister as u8)
                || (data_bytes[0] == Cmd::Write16 as u8)
                || (data_bytes[0] == Cmd::Write32 as u8))
            && data_bytes[1] == b'O'
            && data_bytes[2] == b'K'
        {
            return Ok(ParseOk::Ack);
        }

        // Lazy conversion
        let cmd: Cmd = unsafe { mem::transmute(data_bytes[0]) };
        let addr = u16::from_be_bytes([data_bytes[1], data_bytes[2]]);
        let wlen = data_bytes[3] as usize;
        let data_bytes = &data_bytes[4..];

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
}

fn check_crc16(recv_data: &[u8], recv_crc: u16) -> Result<(), ParseErr> {
    if CRC.checksum(recv_data) != recv_crc {
        return Err(ParseErr::BadCrc);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_with_crc() {
        let parser = Parser::<0x5AA5, 240, true>::new();
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        parser.parse(&packet).expect("Bad parse!");
    }

    #[test]
    fn parse_one_u16() {
        let a = 15u8;
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
