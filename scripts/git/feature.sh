#!/usr/bin/env bash
set -euo pipefail

name="${NAME:-${1:-}}"
if [[ -z "$name" ]]; then
  echo "usage: NAME=<slug> ./scripts/git/feature.sh" >&2
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "working tree is not clean; commit/stash changes before creating a feature branch" >&2
  exit 1
fi

branch="feature/${name}"
git checkout -b "$branch"
echo "created branch: $branch"
