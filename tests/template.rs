use dwin;

#[test]
fn ack_with_crc() {
    let packet = [0x5A, 0xA5, 5, 0x82, 'O' as u8, 'K' as u8, 0xA5, 0xEF];
    let result = dwin::parse(&packet);
    match result.unwrap(){
        dwin::ParseOk::Ack => (),
        _ => assert!(false),
    }
}
