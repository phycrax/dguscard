use criterion::{criterion_group, criterion_main, Criterion};
use dwin::parser::*;

fn parse_one_u16() {
    let mut parser = Parser::<0x5AA5, 64, false>::new();
    let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
    for i in packet {
        if let Some(result) = parser.consume(i) {
            if let ParseResult::Data { addr, .. } = result {
                if addr != 0xAABB {
                    panic!("Wrong adress");
                }
            } else {
                panic!("Expected Data");
            }
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("set_background_icl_output", |b| b.iter(parse_one_u16));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
