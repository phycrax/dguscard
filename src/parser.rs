use crate::{Cmd, Crc16Modbus};

pub struct Parser<const H: u16, const C: bool>;

pub enum ParsedFrame<'a> {
    Ack,
    Data {
        command: Cmd,
        address: u16,
        word_length: u8,
        data_bytes: &'a [u8],
    },
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

impl<const HEADER: u16, const CRC_ENABLED: bool> Parser<HEADER, CRC_ENABLED> {
    pub fn parse(self, bytes: &[u8]) -> Result<ParsedFrame, ParseErr> {
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
        let (length, bytes) = bytes.split_first().ok_or(ParseErr::Length)?;
        if *length as usize != bytes.len() {
            return Err(ParseErr::Length);
        }

        // Strip CRC
        let bytes = if CRC_ENABLED {
            let (crc_h, bytes) = bytes.split_last().ok_or(ParseErr::Checksum)?;
            let (crc_l, bytes) = bytes.split_last().ok_or(ParseErr::Checksum)?;
            if u16::from_be_bytes([*crc_h, *crc_l]) != Self::checksum(bytes) {
                return Err(ParseErr::Checksum);
            }
            bytes
        } else {
            bytes
        };

        // Strip command
        let (command, bytes) = bytes.split_first().ok_or(ParseErr::Command)?;
        let command = Cmd::from(*command);
        if command == Cmd::Undefined {
            return Err(ParseErr::Command);
        }

        // Strip address
        let (addr_h, bytes) = bytes.split_first().ok_or(ParseErr::Address)?;
        let (addr_l, bytes) = bytes.split_first().ok_or(ParseErr::Address)?;
        let address = u16::from_be_bytes([*addr_h, *addr_l]);

        // Is it ACK?
        if bytes.is_empty() {
            if address == u16::from_be_bytes([b'O', b'K']) {
                return Ok(ParsedFrame::Ack);
            } else {
                return Err(ParseErr::Unknown);
            }
        }

        // Strip word length
        let (word_length, data_bytes) = bytes.split_first().ok_or(ParseErr::WordLength)?;
        let word_length = *word_length;

        // Remanining bytes are data
        Ok(ParsedFrame::Data {
            command,
            address,
            word_length,
            data_bytes,
        })
    }
}

#[cfg(feature = "crc")]
impl<const H: u16, const C: bool> Crc16Modbus for Parser<H, C> {
    fn checksum(bytes: &[u8]) -> u16 {
        use crc::{Crc, CRC_16_MODBUS};
        const CRC: crc::Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
        CRC.checksum(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack() {
        let parser = Parser::<0x5AA5, true>;
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        let result = parser.parse(&packet);
        let Ok(ParsedFrame::Ack) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn bad_header() {
        let parser = Parser::<0x5AA5, true>;
        let packet = [0xAA, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        let result = parser.parse(&packet);
        let Err(ParseErr::Header) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn bad_checksum() {
        let parser = Parser::<0x5AA5, true>;
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xAA, 0xEF];
        let result = parser.parse(&packet);
        let Err(ParseErr::Checksum) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn bad_command() {
        let parser = Parser::<0x5AA5, true>;
        let packet = [0x5A, 0xA5, 5, 0xAA, b'O', b'K', 0x25, 0xE7];
        let result = parser.parse(&packet);
        let Err(ParseErr::Command) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn receive_packet() {
        let parser = Parser::<0x5AA5, true>;
        let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];

        let result = parser.parse(&packet).unwrap();

        if let ParsedFrame::Data {
            command,
            address,
            word_length,
            data_bytes,
        } = result
        {
            assert_eq!(command, Cmd::Read16);
            assert_eq!(address, 0xAABB);
            assert_eq!(word_length, 1);
            assert_eq!(&data_bytes, &[0xCC, 0xDD]);
        } else {
            panic!("Shouldn't reach here");
        };
    }
}
