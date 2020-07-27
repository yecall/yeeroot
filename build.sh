#!/usr/bin/env bash
set -e

cp prebuilt/yee_runtime/mainnet.wasm runtime/wasm/target/wasm32-unknown-unknown/release/yee_runtime_wasm.compact.wasm

cargo build --release
