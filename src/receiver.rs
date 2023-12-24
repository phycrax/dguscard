use super::*;

#[derive(Debug)]
pub enum ReceiveError {
    HeaderHigh,
    HeaderLow,
    PayloadLength,
    BufferOverrun,
    BadChecksum,
    BadCommand,
}

enum ReceiverState {
    Initial,
    HeaderHigh,
    HeaderLow,
    Length { length: u8 },
    DataStream { length: u8 },
    ChecksumLow,
    ChecksumHigh { checksum: u8 },
}

pub struct Receiver<const HEADER: u16, const BUFFER_SIZE: usize, const CRC_ENABLED: bool> {
    state: ReceiverState,
    buffer: Vec<u8, BUFFER_SIZE>,
}

pub enum ReceiveOk<const BUFFER_SIZE: usize> {
    Ack,
    Packet {
        cmd: Cmd,
        addr: u16,
        wlen: u8,
        data: Vec<u8, BUFFER_SIZE>,
    },
}

impl<const HEADER: u16, const BUFFER_SIZE: usize, const CRC_ENABLED: bool> Default
    for Receiver<HEADER, BUFFER_SIZE, CRC_ENABLED>
{
    fn default() -> Self {
        assert!(BUFFER_SIZE >= 6, "<BUFFER_SIZE> should be 6 or more");
        assert!(BUFFER_SIZE <= 250, "<BUFFER_SIZE> should be 250 or less");
        Self {
            state: ReceiverState::Initial,
            buffer: Vec::new(),
        }
    }
}

impl<const HEADER: u16, const BUFFER_SIZE: usize, const CRC_ENABLED: bool>
    Receiver<HEADER, BUFFER_SIZE, CRC_ENABLED>
{
    pub fn new() -> Self {
        Default::default()
    }

    const fn check_header_first_byte(byte_in: u8) -> Result<(), ReceiveError> {
        if byte_in == HEADER.to_be_bytes()[0] {
            Ok(())
        } else {
            Err(ReceiveError::HeaderHigh)
        }
    }

    const fn check_header_second_byte(byte_in: u8) -> Result<(), ReceiveError> {
        if byte_in == HEADER.to_be_bytes()[1] {
            Ok(())
        } else {
            Err(ReceiveError::HeaderLow)
        }
    }

    const fn check_length(byte_in: u8) -> Result<(), ReceiveError> {
        let min_len = if CRC_ENABLED { 5 } else { 3 };
        let max_len = BUFFER_SIZE as u8;
        if byte_in >= min_len && byte_in <= max_len {
            Ok(())
        } else {
            Err(ReceiveError::PayloadLength)
        }
    }

    fn check_command(byte_in: u8) -> Result<(), ReceiveError> {
        let cmd: Cmd = byte_in.into();
        if cmd != Cmd::Undefined {
            Ok(())
        } else {
            Err(ReceiveError::BadCommand)
        }
    }

    const fn check_crc16(bytes: &[u8], checksum: u16) -> Result<(), ReceiveError> {
        if checksum == CRC.checksum(bytes) {
            Ok(())
        } else {
            Err(ReceiveError::BadChecksum)
        }
    }

    pub fn parse(&self) -> ReceiveOk<BUFFER_SIZE> {
        let cmd: Cmd = self.buffer[0].into();
        let addr = u16::from_be_bytes([self.buffer[1], self.buffer[2]]);

        if self.buffer.len() == 3 && addr == u16::from_be_bytes([b'O', b'K']) {
            ReceiveOk::Ack
        } else {
            ReceiveOk::Packet {
                cmd,
                addr,
                wlen: self.buffer[3],
                data: Vec::from_slice(&self.buffer[4..]).unwrap(),
            }
        }
    }

    // Async fn possible?
    pub fn consume(&mut self, byte: u8) -> Result<Option<()>, ReceiveError> {
        use ReceiverState::*;
        self.state = match self.state {
            Initial => {
                Self::check_header_first_byte(byte)?;
                HeaderHigh
            }

            HeaderHigh => {
                Self::check_header_second_byte(byte)?;
                HeaderLow
            }

            HeaderLow => {
                Self::check_length(byte)?;
                Length { length: byte }
            }

            Length { length } => {
                Self::check_command(byte)?;
                self.buffer
                    .push(byte)
                    .map_err(|_| ReceiveError::BufferOverrun)?;
                DataStream { length }
            }

            DataStream { length } => {
                self.buffer
                    .push(byte)
                    .map_err(|_| ReceiveError::BufferOverrun)?;
                if CRC_ENABLED && self.buffer.len() == (length - 2) as usize {
                    ChecksumLow
                } else if self.buffer.len() == length as usize {
                    return Ok(Some(()));
                } else {
                    DataStream { length }
                }
            }

            ChecksumLow => ChecksumHigh { checksum: byte },

            ChecksumHigh { checksum } => {
                let checksum = u16::from_le_bytes([checksum, byte]);
                Self::check_crc16(&self.buffer, checksum)?;
                return Ok(Some(()));
            }
        };
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack() {
        let mut receiver = Receiver::<0x5AA5, 8, true>::new();
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];

        packet
            .into_iter()
            .map(|byte| receiver.consume(byte).transpose())
            .find(|f| f.is_some())
            .unwrap()
            .unwrap()
            .unwrap();

        if let ReceiveOk::Ack = receiver.parse() {
        } else {
            panic!("Shouldn't reach here");
        }
    }

    #[test]
    fn ack_nocrc() {
        let mut receiver = Receiver::<0x5AA5, 8, false>::new();
        let packet = [0x5A, 0xA5, 3, 0x82, b'O', b'K'];

        packet
            .into_iter()
            .map(|byte| receiver.consume(byte).transpose())
            .find(|f| f.is_some())
            .unwrap()
            .unwrap()
            .unwrap();

        if let ReceiveOk::Ack = receiver.parse() {
        } else {
            panic!("Shouldn't reach here");
        }
    }

    #[test]
    fn receive_packet() {
        let mut receiver = Receiver::<0x5AA5, 8, true>::new();
        let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];

        packet
            .into_iter()
            .map(|byte| receiver.consume(byte).transpose())
            .find(|f| f.is_some())
            .unwrap()
            .unwrap()
            .unwrap();

        if let ReceiveOk::Packet {
            cmd,
            addr,
            wlen,
            data,
        } = receiver.parse()
        {
            assert_eq!(cmd, Cmd::Read16);
            assert_eq!(addr, 0xAABB);
            assert_eq!(wlen, 1);
            assert_eq!(&data, &[0xCC, 0xDD]);
        } else {
            panic!("Shouldn't reach here");
        };
    }

    #[test]
    fn receive_packet_nocrc() {
        let mut receiver = Receiver::<0x5AA5, 8, false>::new();
        let packet = [0x5A, 0xA5, 6, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD];

        packet
            .into_iter()
            .map(|byte| receiver.consume(byte).transpose())
            .find(|f| f.is_some())
            .unwrap()
            .unwrap()
            .unwrap();

        if let ReceiveOk::Packet {
            cmd,
            addr,
            wlen,
            data,
        } = receiver.parse()
        {
            assert_eq!(cmd, Cmd::Read16);
            assert_eq!(addr, 0xAABB);
            assert_eq!(wlen, 1);
            assert_eq!(&data, &[0xCC, 0xDD]);
        } else {
            panic!("Shouldn't reach here");
        };
    }
}
