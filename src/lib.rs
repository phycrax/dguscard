#![no_std]

//can it be optional?
use crcxx::crc16::{catalog::CRC_16_MODBUS, *};
const CRC: Crc<LookupTable256> = Crc::<LookupTable256>::new(&CRC_16_MODBUS);

//ToDo: make this cfg
const HDR0: u8 = 0x5A;
const HDR1: u8 = 0xA5;
const CRC_ENABLED: bool = true;

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
}

#[derive(Debug)]
pub enum ParseOk {
    Ack,
    Data(Packet),
}

#[derive(Debug)]
pub struct Packet {
    addr: u16,
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

    let data_bytes = &received_bytes[3..len + 1];

    if CRC_ENABLED {
        let received_crc = ((received_bytes[len + 2] as u16) << 8) | received_bytes[len + 1] as u16;
        let calculated_crc = CRC.compute(&data_bytes);
        if calculated_crc != received_crc {
            return Err(ParseErr::BadCrc);
        }
    }

    if len == 3 + CRC_ENABLED as usize * 2
        && (received_bytes[3] == Cmd::VarWrite as u8 || received_bytes[3] == Cmd::VarRead as u8)
        && 'O' == (received_bytes[4] as char)
        && 'K' == (received_bytes[5] as char)
    {
        return Ok(ParseOk::Ack);
    }

    //ToDo: parse address and data
    let packet = Packet { addr: 0x5000 };

    return Ok(ParseOk::Data(packet));
}
