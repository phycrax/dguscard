#!/bin/sh
set -ex

has_target() {
  rustup target list --installed | grep -q "$1"
}
ensure_target() {
  has_target "$1" || rustup target add "$1"
}

ensure_target thumbv6m-none-eabi

has_toolchain() {
  rustup toolchain list | grep -q "$1"
}
ensure_toolchain() {
  has_toolchain "$1" || rustup toolchain install "$1"
}

ensure_toolchain nightly

cargo_check() {
  cargo check --all "$@"
  cargo clippy --all "$@" -- --deny=warnings
}
cargo_test() {
  cargo_check --all-targets "$@"
  cargo test --all "$@"
}

cargo_test

cargo_check --target=thumbv6m-none-eabi
cargo_check --target=thumbv6m-none-eabi --no-default-features

cargo fmt --all -- --check

# Check docs.rs build
env RUSTDOCFLAGS='--cfg=docsrs --deny=warnings' cargo +nightly doc --all --no-deps --all-features
