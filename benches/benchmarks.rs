use criterion::{criterion_group, criterion_main, Criterion};
use dwin::receiver::*;

fn criterion_benchmark(c: &mut Criterion) {
    //c.bench_function("", |b| b.iter());
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
