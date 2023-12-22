use criterion::{criterion_group, criterion_main, Criterion};
use dwin::receiver::*;

fn receive_some_data() {
    let mut receiver = Receiver::<0x5AA5, 64, true>::new();
    let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];
    for i in packet {
        if receiver.consume(i).unwrap().is_some() {
            return;
        }
    }
    // Shouldn't reach here
    panic!("Wrong adress");
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("receive_some_data", |b| b.iter(receive_some_data));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
