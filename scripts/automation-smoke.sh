#!/usr/bin/env bash
set -euo pipefail

echo "[1/5] shell syntax checks"
bash -n \
  scripts/verify.sh \
  scripts/git/feature.sh \
  scripts/git/ship.sh \
  scripts/git/push.sh \
  scripts/git/sync_master.sh \
  scripts/pr/generate_pr.sh

echo "[2/5] PR draft generation smoke"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

worktree_tmp=".tmp"
mkdir -p "$worktree_tmp"
before_count="$(ls -1 "$worktree_tmp"/pr_*.md 2>/dev/null | wc -l)"

TYPE=chore TITLE="automation smoke check" SCOPE=devx ./scripts/pr/generate_pr.sh >/dev/null

after_count="$(ls -1 "$worktree_tmp"/pr_*.md 2>/dev/null | wc -l)"
if [[ "$after_count" -lt "$before_count" ]]; then
  echo "pr-draft generation did not produce expected artifact" >&2
  exit 1
fi

echo "[3/5] ship guard blocks protected branch"
mkdir -p "$tmp_dir/scripts/git"
cp scripts/git/ship.sh "$tmp_dir/scripts/git/ship.sh"
chmod +x "$tmp_dir/scripts/git/ship.sh"

(
  cd "$tmp_dir"
  git init -q -b master
  if MSG="smoke" ./scripts/git/ship.sh >/dev/null 2>&1; then
    echo "ship guard failed: commit on master must be blocked" >&2
    exit 1
  fi
)

echo "[4/5] feature guard blocks dirty tree"
mkdir -p "$tmp_dir/feature_test/scripts/git"
cp scripts/git/feature.sh "$tmp_dir/feature_test/scripts/git/feature.sh"
chmod +x "$tmp_dir/feature_test/scripts/git/feature.sh"

(
  cd "$tmp_dir/feature_test"
  git init -q -b master
  touch dirty.txt
  if NAME=test ./scripts/git/feature.sh >/dev/null 2>&1; then
    echo "feature guard failed: dirty tree must be blocked" >&2
    exit 1
  fi
)

echo "[5/5] justfile parse smoke"
if command -v just >/dev/null; then
  just --list >/dev/null
fi

echo "automation smoke: OK"
