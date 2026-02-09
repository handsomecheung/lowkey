#!/usr/bin/env bash
set -e

current_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
cd "${current_dir}/../.."

rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

echo "Compiled binary located at: target/x86_64-unknown-linux-musl/release/lowkey"
