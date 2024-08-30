# dguscard

[![Documentation](https://docs.rs/dguscard/badge.svg)](https://docs.rs/dguscard)

dguscard is a `#![no_std]` DWIN DGUS2 T5L touchscreen wire format serializer and deserializer for Serde.
It is intended to be a building block for higher level UI elements that can be sent to or received with DWIN DGUS touchscreens.

## Serde Data Model

TODO

## Example - Serialization/Deserialization

dguscard can serialize and deserialize messages similar to other `serde` formats.

Using a `[u8]` buffer to serialize to a `&[u8]`:

```rust
let buf = &mut [0u8; 20];
let expected = &[
    0x5A, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0E, 0xB4,
];
let data = TestTuple::new();

let mut frame = TxFrame::with_slice(buf, Command::WriteVp, 0x00DE).unwrap();
frame.copy_from(&data).unwrap();
let output = frame.finalize(true).unwrap();
assert_eq!(output, expected);
```

Or the optional `heapless` feature to serialize to a `heapless::Vec<u8>`:

```rust
  let expected: Vec<u8, 12> = Vec::from_slice(&[
      0x5A, 0xA5, 9, 0x82, 0x00, 0xDE, 0x5A, 0x00, 0x12, 0x34, 0x0E, 0xB4,
  ])
  .unwrap();
  let data = TestTuple::new();

  let mut frame = TxFrame::with_hvec(Command::WriteVp, 0x00DE).unwrap();
  frame.copy_from(&data).unwrap();
  let output: Vec<u8, 12> = frame.finalize(true).unwrap();
  assert_eq!(output, expected);
```

## Setup - `Cargo.toml`

Don't forget to add [the `no-std` subset](https://serde.rs/no-std.html) of `serde` along with `dguscard` to the `[dependencies]` section of your `Cargo.toml`!

```toml
[dependencies]
dguscard = "0.1.0"

# By default, `serde` has the `std` feature enabled, 
# which makes it unsuitable for embedded targets.
# Disabling default-features fixes this.
serde = { version = "1.0.*", default-features = false }
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
