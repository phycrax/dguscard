use criterion::{criterion_group, criterion_main, Criterion};
use dwin::{parser::*, Cmd};

fn receive_packet() {
    let parser = FrameParser::<0x5AA5, true>;
    let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];

    let result = parser.parse(&packet).unwrap();

    if let ParseOk::Data(mut frame) = result {
        assert_eq!(frame.get_command(), Cmd::Read16);
        assert_eq!(frame.get_address(), 0xAABB);
        assert_eq!(frame.get_u16(), Some(0xCCDD));
        assert_eq!(frame.get_address(), 0xAABC);
        //assert_eq!(word_length, 1);
    } else {
        panic!("Shouldn't reach here");
    };
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("receive_packet", |b| b.iter(receive_packet));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
