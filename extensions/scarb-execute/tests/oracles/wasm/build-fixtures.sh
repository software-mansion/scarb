#!/usr/bin/env sh
set -ex

# Run this script to generate wasm fixtures from their sources.
# Prebuilt fixtures are expected to be commited to the repository.

cd "$(dirname "$0")"

# wasip2 - a real-world-like Rust wasm module using WASI Preview 2
cargo build --manifest-path=fixture-src/wasip2/Cargo.toml --release --target wasm32-wasip2
cp fixture-src/wasip2/target/wasm32-wasip2/release/wasip2.wasm .

# naked - a hand-written WASI Preview 2 component that is structured differently from Rust's output
wasm-tools component embed fixture-src/naked.wit fixture-src/naked.wat | wasm-tools component new -o naked.wasm

# trap - a simple component that checks passing complex types and handling wasm traps in runtime
wasm-tools component embed --dummy fixture-src/trap.wit | wasm-tools component new -o trap.wasm
