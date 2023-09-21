use dwin::{self, *};

#[test]
fn ack_with_crc() {
    let packet = [0x5A, 0xA5, 5, 0x82, b'O', b'K', 0xA5, 0xEF];
    match dwin::parse(&packet) {
        Ok(ParseOk::Ack) => (),
        _ => panic!("Bad parse!"),
    }
}

#[test]
fn parse_one_u16() {
    let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];

    let result = match dwin::parse(&packet) {
        Ok(result) => result,
        _ => panic!("Expected ParseOk"),
    };

    let addr: u16 = match result {
        ParseOk::Data16 { addr, .. } => addr,
        _ => panic!("Expected Data16"),
    };

    if addr != 0xAABB {
        panic!("Wrong adress")
    }
}
