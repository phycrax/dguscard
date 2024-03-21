use crate::{Cmd, Crc16Modbus};

pub struct FrameIterator<'a> {
    command: Cmd,
    address: u16,
    word_length: u8,
    data_bytes: &'a [u8],
}

impl<'a> FrameIterator<'a> {
    pub const fn get_command(&self) -> Cmd {
        self.command
    }

    pub const fn get_address(&self) -> u16 {
        self.address
    }

    pub fn get_u16(&mut self) -> Option<u16> {
        self.get_primitive()
    }
    pub fn get_u32(&mut self) -> Option<u32> {
        self.get_primitive()
    }
    pub fn get_u64(&mut self) -> Option<u64> {
        self.get_primitive()
    }
    pub fn get_i16(&mut self) -> Option<i16> {
        self.get_primitive()
    }
    pub fn get_i32(&mut self) -> Option<i32> {
        self.get_primitive()
    }
    pub fn get_i64(&mut self) -> Option<i64> {
        self.get_primitive()
    }
    pub fn get_f32(&mut self) -> Option<i64> {
        self.get_primitive()
    }
    pub fn get_f64(&mut self) -> Option<i64> {
        self.get_primitive()
    }
}

trait GetPrimitive<T> {
    fn get_primitive(&mut self) -> Option<T>;
}

macro_rules! impl_get_primitive{
    ($($ty:ident)+) => ($(
        impl GetPrimitive<$ty> for FrameIterator<'_> {
            fn get_primitive(&mut self) -> Option<$ty> {
                if self.data_bytes.len() < core::mem::size_of::<$ty>() {
                    return None;
                }
                self.address += core::mem::size_of::<$ty>() as u16 / 2;
                let (bytes, rest) = self.data_bytes.split_at(core::mem::size_of::<$ty>());
                self.data_bytes = rest;
                Some($ty::from_be_bytes(bytes.try_into().unwrap()))
            }
        }
    )+)
}

impl_get_primitive! { u16 i16 u32 i32 u64 i64 f32 f64 }

pub struct FrameParser<const H: u16, const C: bool>;

pub enum ParseOk<'a> {
    Ack,
    Data(FrameIterator<'a>),
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ParseErr {
    Header,
    Length,
    Checksum,
    Command,
    Address,
    Unknown,
    WordLength,
}

impl<const HEADER: u16, const CRC_ENABLED: bool> FrameParser<HEADER, CRC_ENABLED> {
    pub fn parse(bytes: &[u8]) -> Result<ParseOk, ParseErr> {
        // Slice too short?
        let min_len = if CRC_ENABLED { 8 } else { 5 };
        if bytes.len() < min_len {
            return Err(ParseErr::Length);
        }

        // Strip header
        let bytes = bytes
            .strip_prefix(&u16::to_be_bytes(HEADER))
            .ok_or(ParseErr::Header)?;

        // Strip length
        let (length, bytes) = bytes.split_at(1);
        if length[0] as usize != bytes.len() {
            return Err(ParseErr::Length);
        }

        // Strip CRC
        let bytes = if CRC_ENABLED {
            let (bytes, crc) = bytes.split_at(bytes.len() - 2);
            if u16::from_le_bytes(crc.try_into().unwrap()) != Self::checksum(bytes) {
                return Err(ParseErr::Checksum);
            }
            bytes
        } else {
            bytes
        };

        // Strip command
        let (command, bytes) = bytes.split_at(1);
        let command = Cmd::from(command[0]);
        if command == Cmd::Undefined {
            return Err(ParseErr::Command);
        }

        // Strip address
        let (address, bytes) = bytes.split_at(2);
        let address = u16::from_be_bytes(address.try_into().unwrap());

        // Is it ACK?
        if bytes.is_empty() && address == u16::from_be_bytes([b'O', b'K']) {
            return Ok(ParseOk::Ack);
        }

        // Strip word length
        let (word_length, data_bytes) = bytes.split_first().ok_or(ParseErr::WordLength)?;
        let word_length = *word_length;

        // Remanining bytes are data
        Ok(ParseOk::Data(FrameIterator {
            command,
            address,
            word_length,
            data_bytes,
        }))
    }
}

#[cfg(feature = "crc")]
impl<const H: u16, const C: bool> Crc16Modbus for FrameParser<H, C> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack() {
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        let result = FrameParser::<0x5AA5, true>::parse(&packet);
        let Ok(ParseOk::Ack) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn bad_header() {
        let packet = [0xAA, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        let result = FrameParser::<0x5AA5, true>::parse(&packet);
        let Err(ParseErr::Header) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn bad_checksum() {
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xAA, 0xEF];
        let result = FrameParser::<0x5AA5, true>::parse(&packet);
        let Err(ParseErr::Checksum) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn bad_command() {
        let packet = [0x5A, 0xA5, 5, 0xAA, b'O', b'K', 0x25, 0xE7];
        let result = FrameParser::<0x5AA5, true>::parse(&packet);
        let Err(ParseErr::Command) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn receive_packet() {
        let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];

        let result = FrameParser::<0x5AA5, true>::parse(&packet).unwrap();

        if let ParseOk::Data(mut frame) = result {
            assert_eq!(frame.get_command(), Cmd::Read16);
            assert_eq!(frame.get_address(), 0xAABB);
            assert_eq!(frame.get_u16(), Some(0xCCDD));
            assert_eq!(frame.get_address(), 0xAABC);
            //assert_eq!(word_length, 1);
        } else {
            panic!("Shouldn't reach here");
        };
    }
}
