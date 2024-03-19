use crate::CRC;

use super::Cmd;

pub struct Parser<const H: u16, const C: bool>;

pub enum ParsedFrame<'a> {
    Ack,
    Packet {
        command: Cmd,
        address: u16,
        word_length: u8,
        data: &'a [u8],
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

impl Default for Parser<0x5AA5, true> {
    fn default() -> Self {
        Self
    }
}

impl<const H: u16, const C: bool> Parser<H, C> {
    pub fn parse(self, bytes: &[u8]) -> Result<ParsedFrame, ParseErr> {
        // Slice too short?
        let min_len = if C { 8 } else { 5 };
        if bytes.len() < min_len {
            return Err(ParseErr::Length);
        }

        // Strip header
        let bytes = bytes
            .strip_prefix(&u16::to_be_bytes(H))
            .ok_or(ParseErr::Header)?;

        // Strip length
        let (length, bytes) = bytes.split_first().ok_or(ParseErr::Length)?;
        if *length as usize != bytes.len() {
            return Err(ParseErr::Length);
        }

        // Strip CRC
        let bytes = if C {
            let (crc_h, bytes) = bytes.split_last().ok_or(ParseErr::Checksum)?;
            let (crc_l, bytes) = bytes.split_last().ok_or(ParseErr::Checksum)?;
            let crc = u16::from_be_bytes([*crc_h, *crc_l]);

            // todo crc dependency injection to leverage crc hardware?
            if crc != CRC.checksum(bytes) {
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
        let (word_length, bytes) = bytes.split_first().ok_or(ParseErr::WordLength)?;
        let word_length = *word_length;

        // Remanining bytes are data
        Ok(ParsedFrame::Packet {
            command,
            address,
            word_length,
            data: bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test1() {
        let parser = Parser::default();
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
        let result = parser.parse(&packet);
    }

    #[test]
    fn test2() {
        let parser = Parser::default();
        let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
        let result = parser.parse(&packet);
    }
}
