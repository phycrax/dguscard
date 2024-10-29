# dguscard
[![docs.rs](https://docs.rs/dguscard/badge.svg)](https://docs.rs/dguscard)
[![crates.io](https://img.shields.io/crates/d/dguscard.svg)](https://crates.io/crates/dguscard)
[![crates.io](https://img.shields.io/crates/v/dguscard.svg)](https://crates.io/crates/dguscard)

dguscard is a `#![no_std]` [DWIN](https://www.dwin-global.com) T5L DGUS request builder & response parser with [serde](https://serde.rs/), inspired by [postcard](https://github.com/jamesmunns/postcard).

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

## Examples

Take a look at [`Request`](https://docs.rs/dguscard/request/struct.Request.html), [`Response`](https://docs.rs/dguscard/response/struct.Response.html) and [`Accumulator`](https://docs.rs/dguscard/response/struct.Accumulator.html) examples.

### Word Addressing and Big Endianness

T5L is a [word machine](https://en.wikipedia.org/wiki/Word_addressing) and [big-endian](https://en.wikipedia.org/wiki/Endianness). The example below demonstrates how the addressing works. In this example, we are building a request with the values matching their address.

```rust
// Build a write request to address 0x1000
let mut request = Request::with_slice(buf, Word { addr: 0x1000, cmd: Write }).unwrap();

request.push(&0x1000_u16).unwrap();                // @0x1000 1 word
request.push(&0x1001_u16).unwrap();                // @0x1001 1 word
request.push(&0x1002_1003_u32).unwrap();           // @0x1002 2 words
request.push(&0x1004_1005_1006_1007_u64).unwrap(); // @0x1004 4 words
request.push(&0x1008_u16).unwrap();                // @0x1008 1 word
request.push(&0x10_u8).unwrap();                   // @0x1009 half word MSB
request.push(&0x09_u8).unwrap();                   // @0x1009 half word LSB

// Send the request
let frame = request.finalize(true).unwrap();
uart.write(frame);
```

The example below demonstrates how big-endianness work. Consider we sent the write request successfully in the previous example and now we want to read the same data.
```rust
#[derive(serde::Deserialize)]
struct MyData {
    a_msb: u8,
    a_lsb: u8,
    b: u16,
    c: u32,
}

/// Send a 10 word read from address 0x1000 request
let mut request = Request::with_slice(buf, Word { addr: 0x1000, cmd: Read {len: 10} }).unwrap();
let frame = request.finalize(true).unwrap();
uart.write(frame);

/// Read the response
let buf = &mut [0; 50];
uart.read_until_idle(buf);
let mut response = Response::from_bytes(buf, true).unwrap();
let Response::WordData {
    instr:
        Word {
            addr: 0x1000,
            cmd: Read { len: 10 },
        },
    data: response_data,
} = response
else {
    panic!("Unexpected response");
};

let dword: u32 = response_data.take().unwrap();
assert_eq!(dword, 0x1000_1001_u32);

let my_data: MyData = response_data.take().unwrap();
assert_eq!(my_data, 
    MyData { 
        a_msb: 0x10_u8, 
        a_lsb: 0x02_u8, 
        b: 0x1003_u16, 
        c: 0x1004_1005_u32 
    }
);

let bytes: [u8;4] = response_data.take().unwrap();
assert_eq!(my_data, [0x10, 0x06, 0x10, 0x07]);
```

## Serde Data Model Support

- [X] i8, i16, i32, i64, i128 - encoded as big endian
- [X] u8, u16, u32, u64, u128 - encoded as big endian
- [X] f32, f64 - encoded as big endian
- [X] bool - encoded as u16
- [ ] char
- [ ] string
- [X] byte array
- [ ] option
- [X] unit - not encoded
- [X] unit_struct - not encoded
- [ ] unit_variant
- [X] newtype_struct
- [ ] newtype_variant
- [ ] seq
- [X] tuple
- [X] tuple_struct
- [X] tuple_variant
- [ ] map
- [X] struct
- [ ] struct_variant

## License

Licensed under either of
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
at your option.

NOTE: This crate contains portions of [postcard](https://github.com/jamesmunns/postcard) code.
- [LICENSE-APACHE](postcard/LICENSE-APACHE) 
- [LICENSE-MIT](postcard/LICENSE-MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
