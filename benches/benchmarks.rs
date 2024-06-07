use criterion::{criterion_group, criterion_main, Criterion};
use dwin::{
    builder::FrameBuilder,
    parser::{FrameMetadata, FrameParser},
    ser::{to_slice, DwinVariable},
    FrameCommand,
};

fn receive_packet() {
    let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
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

#[derive(serde::Serialize)]
struct BackgroundIcl(u16, u16);

impl BackgroundIcl {
    pub fn new(id: u16) -> Self {
        Self(0x5A00, id)
    }
}

impl DwinVariable for BackgroundIcl {
    const ADDRESS: u16 = 0x00DE;
}

fn set_background_icl_output_new() {
    let mut buf = [0u8; 50];
    let bg = BackgroundIcl::new(0x1234);
    let output = to_slice(&bg, &mut buf, Default::default()).unwrap();

    if output.len() != 12 {
        panic!("Len should have been 12");
    }
    let expected = [
        0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
    ];
    assert_eq!(output, &expected);
}

fn set_background_icl_output_old() {
    let mut buffer = [0u8; 50];
    let mut packet = FrameBuilder::new(
        &mut buffer,
        Default::default(),
        FrameCommand::Write16,
        0x00DE,
    );
    // Example of the pain with number literals, annotation needed.
    packet.append_u16(0x5A00);
    packet.append_u16(0x1234);
    let bytes = packet.consume();

    if bytes.len() != 12 {
        panic!("Len should have been 12");
    }
    let expected = [
        0x5Au8, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0e, 0xb4,
    ];
    assert_eq!(bytes, &expected);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("new", |b| b.iter(set_background_icl_output_new));
    c.bench_function("old", |b| b.iter(set_background_icl_output_old));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
