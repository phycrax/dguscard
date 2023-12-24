use criterion::{criterion_group, criterion_main, Criterion};
use dwin::{receiver::*, Cmd};

fn receive_packet() {
    let mut receiver = Receiver::<0x5AA5, 8, true>::new();
    let packet = [0x5A, 0xA5, 8, 0x83, 0xAA, 0xBB, 1, 0xCC, 0xDD, 0xE7, 0x8D];

    packet
        .into_iter()
        .map(|byte| receiver.consume(byte).transpose())
        .find(|f| f.is_some())
        .unwrap()
        .unwrap()
        .unwrap();

    if let ReceiveOk::Packet {
        cmd,
        addr,
        wlen,
        data,
    } = receiver.parse()
    {
        assert_eq!(cmd, Cmd::Read16);
        assert_eq!(addr, 0xAABB);
        assert_eq!(wlen, 1);
        assert_eq!(&data, &[0xCC, 0xDD]);
    } else {
        panic!("Shouldn't reach here");
    };
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("receive_packet", |b| b.iter(receive_packet));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
