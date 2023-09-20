#![no_std]

//can it be optional?
use crcxx::crc16::{catalog::CRC_16_MODBUS, *};
const CRC: Crc<LookupTable256> = Crc::<LookupTable256>::new(&CRC_16_MODBUS);

//ToDo: make this cfg
const HDR0: u8 = 0x5A;
const HDR1: u8 = 0xA5;
const CRC_ENABLED: bool = false;
const MAX_DATA: usize = 246;

#[repr(u8)]
pub enum Cmd {
    RegWrite = 0x80,
    RegRead,
    VarWrite,
    VarRead,
}

#[derive(Debug)]
pub enum ParseErr {
    BadHdr0,
    BadHdr1,
    BadCrc,
    BadCmd,
}

#[derive(Debug)]
pub enum ParseOk {
    Ack,
    Data(Packet),
}

#[derive(Debug)]
pub struct Packet {
    pub addr: u16,
    pub wlen: usize,
    pub data: [u16; MAX_DATA / 2],
}

//is Result wrap necessary? Should parser be another module?
pub fn parse(received_bytes: &[u8]) -> Result<ParseOk, ParseErr> {
    if HDR0 != received_bytes[0] {
        return Err(ParseErr::BadHdr0);
    }

    if HDR1 != received_bytes[1] {
        return Err(ParseErr::BadHdr1);
    }

    let len: usize = received_bytes[2].into();

    let data_bytes = &received_bytes[3..len + 3 - CRC_ENABLED as usize * 2];

    if CRC_ENABLED {
        let received_crc = u16::from_le_bytes([received_bytes[len + 2], received_bytes[len + 1]]);
        let calculated_crc = CRC.compute(&data_bytes);
        if calculated_crc != received_crc {
            return Err(ParseErr::BadCrc);
        }
    }

    if len == 3 + CRC_ENABLED as usize * 2
        && data_bytes[0] == Cmd::VarWrite as u8
        && data_bytes[1] == 'O' as u8
        && data_bytes[2] == 'K' as u8
    {
        return Ok(ParseOk::Ack);
    }

    let cmd = data_bytes[0];
    let addr = u16::from_be_bytes([data_bytes[1], data_bytes[2]]);
    let wlen = data_bytes[3] as usize;
    let mut data = [0u16; MAX_DATA / 2];

    let data_bytes = &data_bytes[4..];
    if cmd == Cmd::VarRead as u8 {
        // BigEndian u8:2 to native u16
        for (i, bytes) in data_bytes.chunks(2).enumerate() {
            data[i] = u16::from_be_bytes(bytes.try_into().unwrap());
        }
        return Ok(ParseOk::Data(Packet { addr, wlen, data }));
    }

    Err(ParseErr::BadCmd)
}
