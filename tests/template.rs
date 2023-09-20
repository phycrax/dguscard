use dwin::{self, *};

#[test]
fn ack_with_crc() {
    let packet = [0x5A, 0xA5, 5, 0x82, 'O' as u8, 'K' as u8, 0xA5, 0xEF];
    let result = dwin::parse(&packet);
    match result.unwrap() {
        dwin::ParseOk::Ack => (),
        _ => assert!(false),
    }
}

#[test]
fn parse_one_word() {
    let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0x57, 0xE7];
    let result = dwin::parse(&packet).unwrap();
    if let ParseOk::Data(data) = result {
        if data.addr != 0xAABB {
            assert!(false)
        }
    }
}
