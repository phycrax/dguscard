# dguscard
[![docs.rs](https://docs.rs/dguscard/badge.svg)](https://docs.rs/dguscard)
[![crates.io](https://img.shields.io/crates/d/dguscard.svg)](https://crates.io/crates/dguscard)
[![crates.io](https://img.shields.io/crates/v/dguscard.svg)](https://crates.io/crates/dguscard)

dguscard is a `#![no_std]` [DWIN](https://www.dwin-global.com) T5L DGUS request builder & response parser with `serde` capability, inspired by [postcard](https://github.com/jamesmunns/postcard).

## Examples

Take a look at [`Request`](https://docs.rs/dguscard/request/struct.RequestFrame.html), [`Response`](https://docs.rs/dguscard/response/struct.Response.html) and [`Accumulator`](https://docs.rs/dguscard/response/struct.Accumulator.html) examples.

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

NOTE: This crate contains portions of [postcard](https://github.com/jamesmunns/postcard) code.

Postcard copyright notice
- [LICENSE-APACHE](postcard/LICENSE-APACHE) 
- [LICENSE-MIT](postcard/LICENSE-MIT)


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
