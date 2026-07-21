# dguscard
[![docs.rs](https://docs.rs/dguscard/badge.svg)](https://docs.rs/dguscard)
[![crates.io](https://img.shields.io/crates/d/dguscard.svg)](https://crates.io/crates/dguscard)
[![crates.io](https://img.shields.io/crates/v/dguscard.svg)](https://crates.io/crates/dguscard)

A [`postcard`](https://github.com/jamesmunns/postcard)-inspired, `#![no_std]` request builder and response parser for [DWIN](https://www.dwin-global.com) T5L DGUS displays, with [`serde`](https://serde.rs/) support.

## Setup

Add `dguscard` and `serde` (with default features disabled for `no_std` compatibility) to your `[dependencies]`:

```toml
[dependencies]
dguscard = "0.1.0"

# By default, `serde` has the `std` feature enabled, 
# which makes it unsuitable for embedded targets.
# Disabling default-features fixes this.
serde = { version = "1.0.*", default-features = false }
```

## Usage

See the documented examples on [`Request`](https://docs.rs/dguscard/latest/dguscard/request/struct.Request.html), [`Response`](https://docs.rs/dguscard/latest/dguscard/response/enum.Response.html), and [`Accumulator`](https://docs.rs/dguscard/latest/dguscard/response/struct.Accumulator.html).

### Write and Read

T5L is a [word-addressed](https://en.wikipedia.org/wiki/Word_addressing), [big-endian](https://en.wikipedia.org/wiki/Endianness) device. The examples below write data to the display and read it back.

```rust
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct MyData {
    a_msb: u8,
    a_lsb: u8,
    b: u16,
    c: u32,
}

// Backing buffer for the request.
let tx_buf = &mut [0u8; 50];

// Build a write request to address 0x1000
let mut request = Request::with_slice(tx_buf, Address::Word(0x1000), Write).unwrap();

request.push(&0x1000_u16).unwrap();                                         // @0x1000 1 word
request.push(&0x1001_1002_u32).unwrap();                                    // @0x1001 2 words
request.push(&0x1003_1004_1005_1006_u64).unwrap();                          // @0x1003 4 words
let data = MyData { a_msb: 0x10, a_lsb: 0x07, b: 0x1008, c: 0x1009_100A };
request.push(&data).unwrap();                                               // @0x1007 4 words
request.push(&0x10_u8).unwrap();                                            // @0x100B half word MSB
request.push(&0x0B_u8).unwrap();                                            // @0x100B half word LSB

// Finalize with CRC and send
let frame = request.finalize(true).unwrap();
uart.write(frame);
```

Read the data back:

```rust
// Read back the 12 words we just wrote, starting at address 0x1000
let mut request = Request::with_slice(tx_buf, Address::Word(0x1000), Read(12)).unwrap();
let frame = request.finalize(true).unwrap();
uart.write(frame);

// Backing buffer for the response.
let rx_buf = &mut [0u8; 50];
uart.read_until_idle(rx_buf);
let mut response = Response::from_bytes(rx_buf, true).unwrap();
let Response::Data {
    addr,
    wlen,
    mut content,
} = response
else {
    panic!("Unexpected response type");
};

assert_eq!(addr, Address::Word(0x1000));
assert_eq!(wlen, 12);

// Take the values back in the same order and types they were pushed.
let word: u16 = content.take().unwrap();                    // @0x1000 1 word
assert_eq!(word, 0x1000_u16);

let dword: u32 = content.take().unwrap();                   // @0x1001 2 words
assert_eq!(dword, 0x1001_1002_u32);

let qword: u64 = content.take().unwrap();                   // @0x1003 4 words
assert_eq!(qword, 0x1003_1004_1005_1006_u64);

let my_data: MyData = content.take().unwrap();              // @0x1007 4 words
assert_eq!(my_data,
    MyData {
        a_msb: 0x10_u8,
        a_lsb: 0x07_u8,
        b: 0x1008_u16,
        c: 0x1009_100A_u32
    }
);

let msb: u8 = content.take().unwrap();                      // @0x100B half word MSB
let lsb: u8 = content.take().unwrap();                      // @0x100B half word LSB
assert_eq!((msb, lsb), (0x10, 0x0B));
```

## Serde Data Model Support

- ✅ i8, i16, i32, i64, i128 - encoded as big endian
- ✅ u8, u16, u32, u64, u128 - encoded as big endian
- ✅ f32, f64 - encoded as big endian
- ✅ bool - encoded as u16
- ❌ char
- ❌ string
- ❌ byte array - serialization only
- ❌ option
- ✅ unit - not encoded
- ✅ unit_struct - not encoded
- ✅ unit_variant - index encoded as u16
- ✅ newtype_struct
- ❌ newtype_variant
- ❌ seq
- ✅ tuple
- ✅ tuple_struct
- ❌ tuple_variant
- ❌ map
- ✅ struct
- ❌ struct_variant

## License

Licensed under either of
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
at your option.

Portions of this crate are derived from [postcard](https://github.com/jamesmunns/postcard),
Copyright (c) 2019 Anthony James Munns, licensed under MIT OR Apache-2.0.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
