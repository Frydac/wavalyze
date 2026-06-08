#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

cargo +1.93.0 check --quiet --workspace --all-targets
RUSTUP_TOOLCHAIN=nightly-2026-01-15 cargo check --quiet --workspace --all-features --lib --target wasm32-unknown-unknown
cargo +1.93.0 fmt --all -- --check
cargo +1.93.0 clippy --quiet --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo +1.93.0 test --quiet --workspace --all-targets --all-features
cargo +1.93.0 test --quiet --workspace --doc
env -u NO_COLOR ./scripts/trunk-threaded.sh build
