use crate::{Config, Crc16Modbus, FrameCommand};

#[derive(PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FrameMetadata {
    pub command: FrameCommand,
    pub address: u16,
    pub word_length: u8,
}

#[derive(PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FrameData<'a>(&'a [u8]);

#[derive(PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Frame<'a> {
    metadata: FrameMetadata,
    data: FrameData<'a>,
}

impl<'a> Frame<'a> {
    pub fn is_ack(&self) -> bool {
        self.data.is_empty() && self.metadata.address == u16::from_be_bytes([b'O', b'K'])
    }
    pub fn metadata(&self) -> FrameMetadata {
        self.metadata
    }
    pub fn data(&self) -> FrameData {
        self.data
    }
}


#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ParseErr {
    Header,
    Length,
    Checksum,
    Command,
    Address,
    Unknown,
    WordLength,
}

pub struct FrameParser<T> {
    config: Config<T>,
}

impl<T: Crc16Modbus> FrameParser<T> {
    pub fn new(config: Config<T>) -> Self {
        Self { config }
    }
    // Maybe consider returning multiple errors?
    // CRC will always be invalid, would be good to know what got corrupted?
    pub fn parse<'a>(&'a self, bytes: &'a [u8]) -> Result<Frame, ParseErr> {
        // Slice too short?
        let min_len = if self.config.crc { 8 } else { 6 };
        if bytes.len() < min_len {
            return Err(ParseErr::Length);
        }

        // Strip header
        let bytes = bytes
            .strip_prefix(&u16::to_be_bytes(self.config.header))
            .ok_or(ParseErr::Header)?;

        // Strip length
        let (length, bytes) = bytes.split_first().unwrap();
        let length = *length as usize;

        // Trim slice with the length
        let bytes = bytes.get(..length).ok_or(ParseErr::Length)?;

        // Strip CRC
        let bytes = if self.config.crc {
            let (bytes, crc) = bytes.split_last_chunk().unwrap();
            if u16::from_le_bytes(*crc) != self.config.crc_engine.checksum(bytes) {
                return Err(ParseErr::Checksum);
            }
            bytes
        } else {
            bytes
        };

        // Strip command
        let (command, bytes) = bytes.split_first().unwrap();
        let command = FrameCommand::from(*command);
        if command == FrameCommand::Undefined {
            return Err(ParseErr::Command);
        }

        // Strip address
        let (address, bytes) = bytes.split_first_chunk().unwrap();
        let address = u16::from_be_bytes(*address);

        // Strip word length, if there is none (in case it's ACK), set to 0
        let (word_length, bytes) = bytes.split_first().unwrap_or((&0, bytes));
        let word_length = *word_length;

        let metadata = FrameMetadata {
            command,
            address,
            word_length,
        };
        let data = FrameData(bytes);
        let frame = Frame { metadata, data };

        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack() {
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];
        let parser = FrameParser::new(Default::default());
        let frame = parser.parse(&packet).expect("Parsing failure");
        if !frame.is_ack() {
            panic!("Not ACK");
        };
    }

    #[test]
    fn bad_header() {
        let packet = [0xAA, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF, 0, 0, 0, 0];
        let parser = FrameParser::new(Default::default());
        let result = parser.parse(&packet);
        let Err(ParseErr::Header) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn bad_checksum() {
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xAA, 0xEF, 0, 0, 0, 0];
        let parser = FrameParser::new(Default::default());
        let result = parser.parse(&packet);
        let Err(ParseErr::Checksum) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn bad_command() {
        let packet = [0x5A, 0xA5, 5, 0xAA, b'O', b'K', 0x25, 0xE7, 0, 0, 0, 0];
        let parser = FrameParser::new(Default::default());
        let result = parser.parse(&packet);
        let Err(ParseErr::Command) = result else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn receive_packet() {
        let packet = [
            0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D, 0, 0, 0, 0,
        ];
        let expected_metadata = FrameMetadata {
            command: FrameCommand::Read16,
            address: 0xAABB,
            word_length: 1,
        };
        let parser = FrameParser::new(Default::default());
        let frame = parser.parse(&packet).expect("Parsing failure");
        assert_eq!(frame.metadata(), expected_metadata);
        assert_eq!(frame.data().get_u16(), Some(0xCCDD));
    }
}
