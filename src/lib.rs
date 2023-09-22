#![no_std]

use core::mem;

//can it be optional?
use crcxx::crc16::{catalog::CRC_16_MODBUS, *};
const CRC: Crc<LookupTable256> = Crc::<LookupTable256>::new(&CRC_16_MODBUS);

//ToDo: make this cfg
const HDR0: u8 = 0x5A;
const HDR1: u8 = 0xA5;
const CRC_ENABLED: bool = true;
const MAX_DATA: usize = 246;

#[repr(u8)]
pub enum Cmd {
    Write8 = 0x80,
    Read8,
    Write16,
    Read16,
    WriteCurve,
    Undefined,
    Write32,
    Read32,
}

pub enum ParseErr {
    BadHdr0,
    BadHdr1,
    BadCrc,
    BadCmd,
}

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

//is Result wrap necessary? Should parser be another module?
pub fn parse(received_bytes: &[u8]) -> Result<ParseOk, ParseErr> {
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

    // Check if headers are correct
    if HDR0 != received_bytes[0] {
        return Err(ParseErr::BadHdr0);
    }

    if HDR1 != received_bytes[1] {
        return Err(ParseErr::BadHdr1);
    }

    // Get the packet length including as usize, rust limitation
    let len = received_bytes[2] as usize;

    // Slice between LEN and CRC
    let data_bytes = &received_bytes[3..len + 3 - CRC_ENABLED as usize * 2];

    // Calculate CRC16 if enabled
    if CRC_ENABLED {
        let received_crc = u16::from_le_bytes([received_bytes[len + 1], received_bytes[len + 2]]);
        if CRC.compute(data_bytes) == received_crc {
            return Err(ParseErr::BadCrc);
        }
    }

    // Is it ack?
    if len == 3 + CRC_ENABLED as usize * 2
        && ((data_bytes[0] == Cmd::Write8 as u8)
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
        Cmd::Read8 => {
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

// todo: move to separate file
pub struct Packet {
    data: [u8; MAX_DATA],
}

impl Packet {
    //todo: only accept write cmds, return error otherwise
    pub fn build(cmd: Cmd, addr: u16) -> Packet {
        let mut packet = Packet {
            data: [0; MAX_DATA],
        };
        packet.data[0] = HDR0;
        packet.data[1] = HDR1;
        // index 2 is skipped, it tracks len
        packet.add(cmd as u8); //index 3
        packet.add_u16(addr); //index 4 and 5
        packet
    }

    // todo: test dwin response if len is oddnum?
    // Note: Consumes the package, package invalid afterwards
    pub fn consume(mut self) -> (usize, [u8; MAX_DATA]) {
        if CRC_ENABLED {
            // calculate crc from [CMD] to end.
            let len = self.data[2] as usize + 3;
            let data_bytes = &self.data[3..len];
            let crc = CRC.compute(data_bytes).to_le_bytes();
            // CRC should be little endian in payload, so can't use add_u16
            self.add(crc[0]);
            self.add(crc[1]);
        }

        // actual payload len is len + 3
        // +2 header and +1 len itself
        // todo: 3 scattered everywhere RN, consider making it const
        let len = self.data[2] as usize + 3;
        (len, self.data)
    }

    fn add(&mut self, data: u8) {
        // 2nd index in payload represents length excluding header and itself
        // we use this directly to track payload length
        // skip first three index - header and itself
        self.data[self.data[2] as usize + 3] = data;
        self.data[2] += 1;
    }

    // todo: try generics?
    pub fn add_u8(&mut self, data: u8) {
        let bytes = data.to_be_bytes();
        for byte in bytes {
            self.add(byte);
        }
    }

    pub fn add_u16(&mut self, data: u16) {
        let bytes = data.to_be_bytes();
        for byte in bytes {
            self.add(byte);
        }
    }

    pub fn add_u32(&mut self, data: u32) {
        let bytes = data.to_be_bytes();
        for byte in bytes {
            self.add(byte);
        }
    }
}
