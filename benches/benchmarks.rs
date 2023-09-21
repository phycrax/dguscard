use criterion::{criterion_group, criterion_main, Criterion};
use dwin::{self, *};

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

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("parse_one_word", |b| b.iter(parse_one_u16));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
