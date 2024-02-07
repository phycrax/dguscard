pub mod receiver_state;

use core::num::Wrapping;

use self::receiver_state::ReceiverState;
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

pub struct Receiver<const HEADER: u16, const BUFFER_SIZE: usize, const CRC_ENABLED: bool> {
    state: ReceiverState,
    buffer: Vec<u8, BUFFER_SIZE>,
    ack_counter: Wrapping<u8>,
}

pub enum ReceiveOk<'a, const BUFFER_SIZE: usize> {
    Ack,
    Packet {
        cmd: Cmd,
        addr: u16,
        wlen: u8,
        data: &'a [u8],
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
            ack_counter: Wrapping::default(),
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

    pub fn parse(buffer: &[u8]) -> ReceiveOk<BUFFER_SIZE> {
        let cmd: Cmd = buffer[0].into();
        let addr = u16::from_be_bytes([buffer[1], buffer[2]]);

        if buffer.len() == 3 && addr == u16::from_be_bytes([b'O', b'K']) {
            ReceiveOk::Ack
        } else {
            ReceiveOk::Packet {
                cmd,
                addr,
                wlen: buffer[3],
                data: &buffer[4..],
            }
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.state = ReceiverState::Initial;
    }

    pub fn parse_byte(&mut self, byte: u8) -> Result<Option<()>, ReceiveError> {
        use ReceiverState::*;
        match self.state {
            HeaderHigh => {
                Self::check_header_first_byte(byte)?;
            }

            HeaderLow => {
                Self::check_header_second_byte(byte)?;
            }

            Length { .. } => {
                Self::check_length(byte)?;
            }

            Command { .. } => {
                Self::check_command(byte)?;
                self.buffer
                    .push(byte)
                    .map_err(|_| ReceiveError::BufferOverrun)?;
            }

            DataStream { length } => {
                self.buffer
                    .push(byte)
                    .map_err(|_| ReceiveError::BufferOverrun)?;
                if length == 0 {
                    if CRC_ENABLED {
                        let checksum = u16::from_be_bytes([
                            self.buffer.pop().unwrap(),
                            self.buffer.pop().unwrap(),
                        ]);
                        Self::check_crc16(&self.buffer, checksum)?;
                        return Ok(Some(()));
                    } else {
                        return Ok(Some(()));
                    }
                }
            }
            _ => panic!(),
        }
        Ok(None)
    }

    // Async fn possible?
    pub fn consume(&mut self, byte: u8) -> Option<Result<(), ReceiveError>> {
        // TODO: start a watchdog timer for timeout detection here
        //
        // self.watchdog.start();
        // self.watchdog.reset();

        // A byte received, move to the next state.
        self.state = self.state.next(byte);
        // Parse the incoming byte with this state. Return early if there's nothing yet.
        // If there's a result, map it with the error or parse result.
        let result = self
            .parse_byte(byte)
            .transpose()?
            .map(|()| Self::parse(&self.buffer));
        // At this point, we either have a parsed result or an error.

        match result {
            Ok(ReceiveOk::Ack) => self.ack_counter += 1,

            Ok(ReceiveOk::Packet {
                cmd,
                addr,
                wlen,
                data,
            }) => (),

            _ => (),
        }

        // Reset the receiver.
        self.reset();
        // TODO: stop the timer.
        // self.watchdog.stop();

        // Return the result
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack() {
        let mut receiver = Receiver::<0x5AA5, 8, true>::new();
        let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];

        let result = packet
            .into_iter()
            .map(|byte| receiver.consume(byte))
            .find(|f| f.is_some())
            .unwrap()
            .unwrap()
            .unwrap();

        if let ReceiveOk::Ack = result {
        } else {
            panic!("Shouldn't reach here");
        }
    }

    #[test]
    fn ack_nocrc() {
        let mut receiver = Receiver::<0x5AA5, 8, false>::new();
        let packet = [0x5A, 0xA5, 3, 0x82, b'O', b'K'];

        let result = packet
            .into_iter()
            .map(|byte| receiver.consume(byte))
            .find(|f| f.is_some())
            .unwrap()
            .unwrap()
            .unwrap();

        if let ReceiveOk::Ack = result {
        } else {
            panic!("Shouldn't reach here");
        }
    }

    #[test]
    fn receive_packet() {
        let mut receiver = Receiver::<0x5AA5, 8, true>::new();
        let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];

        let result = packet
            .into_iter()
            .map(|byte| receiver.consume(byte))
            .find(|f| f.is_some())
            .unwrap()
            .unwrap()
            .unwrap();

        if let ReceiveOk::Packet {
            cmd,
            addr,
            wlen,
            data,
        } = result
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

        let result = packet
            .into_iter()
            .map(|byte| receiver.consume(byte))
            .find(|f| f.is_some())
            .unwrap()
            .unwrap()
            .unwrap();

        if let ReceiveOk::Packet {
            cmd,
            addr,
            wlen,
            data,
        } = result
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
