use super::*;
use core::mem;

#[derive(Debug)]
pub enum ParseErr {
    BadHdr0,
    BadHdr1,
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

pub struct Parser {
    config: Config,
}

impl Parser {
    pub fn new(config: &Config) -> Parser {
        Parser {
            config: config.clone(),
        }
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
    pub fn parse(&self, received_bytes: &[u8]) -> Result<ParseOk, ParseErr> {
        check_headers(&self.config, &received_bytes[..2])?;
        // Get the packet length including as usize, rust limitation
        let len = received_bytes[2] as usize;

        // Slice between LEN and CRC
        let data_bytes = &received_bytes[3..len + 3 - self.config.crc_enabled as usize * 2];

        // Calculate CRC16 if enabled
        if self.config.crc_enabled {
            let recv_crc = u16::from_le_bytes([received_bytes[len + 1], received_bytes[len + 2]]);
            check_crc16(data_bytes, recv_crc)?;
        }

        // Is it ack?
        if len == 3 + self.config.crc_enabled as usize * 2
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

fn check_headers(config: &Config, frame: &[u8]) -> Result<(), ParseErr> {
    if config.header1 != frame[0] {
        return Err(ParseErr::BadHdr0);
    }
    if config.header2 != frame[1] {
        return Err(ParseErr::BadHdr1);
    }
    Ok(())
}

fn check_crc16(recv_data: &[u8], recv_crc: u16) -> Result<(), ParseErr> {
    if CRC.compute(recv_data) != recv_crc {
        return Err(ParseErr::BadCrc);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_with_crc() {
        let parser = Parser::new(&Config {
            header1: 0x5A,
            header2: 0xA5,
            crc_enabled: true,
        });
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        parser.parse(&packet).expect("Bad parse!");
    }

    #[test]
    fn parse_one_u16() {
        let parser = Parser::new(&Config {
            header1: 0x5A,
            header2: 0xA5,
            crc_enabled: true,
        });
        let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
        let result = parser.parse(&packet).expect("Expected ParseOk, received");

        if let ParseOk::Data16 { addr, .. } = result {
            if addr != 0xAABB {
                panic!("Wrong adress");
            }
        } else {
            panic!("Expected Data16");
        }
    }
}
