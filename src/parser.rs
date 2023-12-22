pub enum ReceiveSuccess {
    Ack,
    Data { cmd: Cmd, addr: u16, wlen: u8 },
}

// const fn is_ack(len: usize, addr: u16) -> bool {
//     let is_ack_len = if CRC_ENABLED { len == 5 } else { len == 3 };
//     is_ack_len && addr == u16::from_le_bytes([b'O', b'K'])
// }

// fn parse(bytes: &[u8]) -> Result<Option<ReceiveSuccess>, ReceiveError> {
//     let cmd: Cmd = bytes[0].into();
//     if cmd == Cmd::Undefined {
//         return Err(ReceiveError::BadCmd);
//     }

//     let addr = u16::from_be_bytes([bytes[1], bytes[2]]);

//     if Self::is_ack(bytes.len(), addr) {
//         return Ok(Some(ReceiveSuccess::Ack));
//     }

//     Ok(Some(ReceiveSuccess::Data {
//         wlen: bytes[3],
//         addr,
//         cmd,
//     }))
// }
