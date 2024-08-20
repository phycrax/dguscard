# serde_dgus

[![Documentation](https://docs.rs/serde_dgus/badge.svg)](https://docs.rs/serde_dgus)

serde_dgus is a `#![no_std]` serializer and deserializer for DWIN DGUS touchscreen wire format. 
It is intended to be a building block for higher level UI libraries that need to communicate with DWIN DGUS touchscreens.

## Example - Serialization/Deserialization

serde_dgus can serialize and deserialize messages similar to other `serde` formats.

Using a `[u8]` buffer to serialize to a `&[u8]`:

```rust

```

Or the optional `heapless` feature to serialize to a `heapless::Vec<u8>`:

```rust

```

## Setup - `Cargo.toml`

Don't forget to add [the `no-std` subset](https://serde.rs/no-std.html) of `serde` along with `serde_dgus` to the `[dependencies]` section of your `Cargo.toml`!

```toml
[dependencies]
serde_dgus = "0.1.0"

# By default, `serde` has the `std` feature enabled, which makes it unsuitable for embedded targets
# disabling default-features fixes this
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
