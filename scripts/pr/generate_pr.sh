#!/usr/bin/env bash
set -euo pipefail

type="${TYPE:-${1:-}}"
title_input="${TITLE:-${2:-}}"
scope="${SCOPE:-${3:-}}"

if [[ -z "$type" || -z "$title_input" ]]; then
  echo "usage: TYPE=<feat|fix|docs|chore> TITLE=\"...\" [SCOPE=\"...\"] ./scripts/pr/generate_pr.sh" >&2
  exit 1
fi

case "$type" in
  feat|fix|docs|chore|refactor|test|ci) ;;
  *)
    echo "unsupported TYPE: $type" >&2
    exit 1
    ;;
esac

branch="$(git branch --show-current 2>/dev/null || true)"
if [[ -z "$branch" ]]; then
  branch="${GITHUB_HEAD_REF:-${GITHUB_REF_NAME:-}}"
fi
if [[ -z "$branch" ]]; then
  branch="detached-head"
fi

if [[ -n "$scope" ]]; then
  pr_title="$type($scope): $title_input"
else
  pr_title="$type: $title_input"
fi

mkdir -p .tmp
outfile=".tmp/pr_${branch//\//_}.md"

cat > "$outfile" <<PR
# Title
$pr_title

# Description
## Summary
- TODO: briefly describe the user-visible outcome.

## Changes
- TODO: list key code/documentation changes.

## Validation
- [ ] cargo fmt --check
- [ ] cargo clippy --all-targets --all-features -- -D warnings
- [ ] cargo test
- [ ] e2e (if applicable)

## Docs
- [ ] Updated docs if behavior or process changed

## Risks / Notes
- TODO: compatibility risks, follow-ups, or known limitations.
PR

echo "PR draft written to: $outfile"
echo ""
cat "$outfile"
