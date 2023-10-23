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

#[test]
fn set_background_icl_output() {
    let mut packet = Packet::new(Cmd::Write16, 0x00DE);
    packet.append(0x5A00_u16);
    packet.append(0x1234_u16);
    let (len, data) = packet.consume();

    if len != 12 {
        panic!("Len should have been 12");
    }

    let test_output = [
        0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
    ];

    for i in 0..12 {
        assert!(
            test_output[i] == data[i],
            "Expected:{} Received:{} At Index:{}",
            test_output[i],
            data[i],
            i
        );
    }
}
