# dguscard

[![Documentation](https://docs.rs/dguscard/badge.svg)](https://docs.rs/dguscard)

dguscard is a [postcard](https://github.com/jamesmunns/postcard)ish `#![no_std]` DWIN DGUS2 T5L touchscreen wire format frame parser/builder with `serde` capability.
It is intended to be a building block for higher level UI elements that can be sent to or received with DWIN DGUS touchscreens.

## Examples

Take a look at [`rx::RxFrame`](https://docs.rs/dguscard/rx/struct.Accumulator.html), [`rx::Accumulator`](https://docs.rs/dguscard/rx/struct.RxFrame.html) and [`tx::TxFrame`](https://docs.rs/dguscard/tx/struct.TxFrame.html) examples.

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
- [LICENSE-APACHE](postcard-LICENSE-APACHE) 
- [LICENSE-MIT](postcard-LICENSE-MIT)


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
