use criterion::{criterion_group, criterion_main, Criterion};
use dwin::{self, *};

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

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("set_background_icl_output", |b| {
        b.iter(set_background_icl_output)
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
