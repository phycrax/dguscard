use criterion::{criterion_group, criterion_main, Criterion};
use dwin::{
    builder::FrameBuilder,
    parser::{FrameMetadata, FrameParser},
    FrameCommand,
};

fn receive_packet() {
    let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
    let expected_metadata = FrameMetadata {
        command: FrameCommand::Read16,
        address: 0xAABB,
        word_length: 1,
    };

    let frame = FrameParser::<0x5AA5, true>::parse(&packet).expect("Parsing failure");
    assert_eq!(frame.metadata(), expected_metadata);
    assert_eq!(frame.data().get_u16(), Some(0xCCDD));
}

fn set_background_icl_output() {
    let mut packet = FrameBuilder::<50, 0x5AA5, true>::new(FrameCommand::Write16, 0x00DE);

    // Example of the pain with number literals, annotation needed.
    packet.append_u16(0x5A00);
    packet.append_u16(0x1234);
    let bytes = packet.get();

    if bytes.len() != 12 {
        panic!("Len should have been 12");
    }

    let test_output = [
        0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
    ];

    assert_eq!(bytes, &test_output);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("receive_packet", |b| b.iter(receive_packet));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
