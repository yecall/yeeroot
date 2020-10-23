#!/usr/bin/env bash
set -e
if [ "$1" == "test" ];then
  cp -f prebuilt/yee_runtime/upgrade/mainnet_v5.wasm runtime/wasm/target/wasm32-unknown-unknown/release/yee_runtime_wasm.compact.wasm
else
  cp -f prebuilt/yee_runtime/mainnet.wasm runtime/wasm/target/wasm32-unknown-unknown/release/yee_runtime_wasm.compact.wasm
fi
cargo build --release
