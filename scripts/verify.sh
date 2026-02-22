#!/usr/bin/env bash
set -euo pipefail

echo "[1/3] rustfmt --check (changed files only)"
base_ref="${VERIFY_BASE_REF:-origin/master}"
if git rev-parse --verify --quiet "$base_ref" >/dev/null; then
  mapfile -t rs_files < <(git diff --name-only --diff-filter=ACMRT "${base_ref}...HEAD" -- '*.rs')
else
  mapfile -t rs_files < <(git diff --name-only --diff-filter=ACMRT -- '*.rs')
fi

if [[ "${#rs_files[@]}" -gt 0 ]]; then
  rustfmt --check "${rs_files[@]}"
else
  echo "no changed Rust files; skipping rustfmt check"
fi

echo "[2/3] cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "[3/3] cargo test"
cargo test

echo "verify: OK"
