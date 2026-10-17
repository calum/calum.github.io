#!/bin/sh
# Compiles the art to WebAssembly and puts it next to index.html as art.wasm.
# One-time setup: rustup target add wasm32-unknown-unknown
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/site_art.wasm ../art.wasm
ls -l ../art.wasm
