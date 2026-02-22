#!/usr/bin/env bash
set -euo pipefail

echo "[1/3] cargo fmt --check"
cargo fmt --check

echo "[2/3] cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "[3/3] cargo test"
cargo test

echo "verify: OK"
