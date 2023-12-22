use super::*;

#[derive(Debug)]
pub enum ReceiveError {
    BadHdr0,
    BadHdr1,
    BadLen,
    Overrun,
    BadCrc,
    BadCmd,
}

enum ReceiverState {
    WaitingHeader,
    ReceivingHeader,
    WaitingLength,
    WaitingCommand,
    WaitingAddress { firstByte: u8 },
    ReceivingAddress,
    WaitingWordLength,
    ReceivingData { received: u8 },
    WaitingChecksum { firstByte: u8 },
    ReceivingChecksum,
    Done,
}

pub struct Receiver<const HEADER: u16, const CRC_ENABLED: bool> {
    state: ReceiverState,
}

impl<const HEADER: u16, const CRC_ENABLED: bool> Default for Receiver<HEADER, CRC_ENABLED> {
    fn default() -> Self {
        Self {
            state: ReceiverState::WaitingHeader,
        }
    }
}

impl<const HEADER: u16, const CRC_ENABLED: bool> Receiver<HEADER, CRC_ENABLED> {
    pub fn new() -> Self {
        Default::default()
    }

    const fn check_header_first_byte(byte_in: u8) -> Result<(), ReceiveError> {
        if byte_in == HEADER.to_be_bytes()[0] {
            Ok(())
        } else {
            Err(ReceiveError::BadHdr0)
        }
    }

    const fn check_header_second_byte(byte_in: u8) -> Result<(), ReceiveError> {
        if byte_in == HEADER.to_be_bytes()[1] {
            Ok(())
        } else {
            Err(ReceiveError::BadHdr1)
        }
    }

    const fn check_length(byte_in: u8) -> Result<(), ReceiveError> {
        let min_len = if CRC_ENABLED { 5 } else { 3 };
        let max_len = 246;
        if byte_in >= min_len && byte_in <= max_len {
            Ok(())
        } else {
            Err(ReceiveError::BadLen)
        }
    }

    fn check_crc16(bytes: &[u8]) -> Result<(), ReceiveError> {
        let len = bytes.len();
        let recv_crc = u16::from_le_bytes([bytes[len - 2], bytes[len - 1]]);
        if recv_crc == CRC.checksum(&bytes[..len - 2]) {
            Ok(())
        } else {
            Err(ReceiveError::BadCrc)
        }
    }

    // Async fn possible?
    pub fn consume(&mut self, packet: &mut Packet, byte: u8) -> Result<Option<()>, ReceiveError> {
        use ReceiverState::*;
        match self.state {
            WaitingHeader => {
                Self::check_header_first_byte(byte)?;
                self.state = ReceivingHeader;
            }
            ReceivingHeader => {
                Self::check_header_second_byte(byte)?;
                self.state = WaitingLength;
            }
            WaitingLength => {
                Self::check_length(byte)?;
                self.state = ReceivingData {
                    length: byte as usize,
                    received: 0,
                };
            }
            ReceivingData { ref mut received } => {
                let index = buffer.get_mut(*received).ok_or(ReceiveError::Overrun)?;
                *index = byte;
                *received += 1;
                if *received == length {
                    if CRC_ENABLED {
                        Self::check_crc16(&buffer[..length])?;
                    }
                    return Ok(Some(()));
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //#[test]
    // fn ack_with_crc() {
    //     let parser = Parser::<0x5AA5, 240, true>::new();
    //     let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
    //     parser.parse(&packet).expect("Bad parse!");
    // }
    #[test]
    fn receive_some_data() {
        let mut receiver = Receiver::<0x5AA5, true>::new();
        let packet: [u8; 11] = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
        let expected_buffer_content: [u8; 6] = [0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD];
        let mut buffer: [u8; 6] = [0; 6];
        for byte in packet {
            if receiver.consume(&mut buffer, byte).unwrap().is_some() {
                assert_eq!(buffer, expected_buffer_content);
                return;
            }
        }
        // Shouldn't reach here
        panic!("Wrong adress");
    }
}
