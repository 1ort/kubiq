#!/usr/bin/env bash
set -euo pipefail

required_docs=(
  "README.md"
  "docs/overview.md"
  "docs/development/setup.md"
  "docs/development/workflow.md"
)

echo "[docs-check] required files exist"
for file in "${required_docs[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "missing required docs file: $file" >&2
    exit 1
  fi
done

echo "[docs-check] workflow doc is linked from overview"
if ! rg -q "development/workflow\\.md" docs/overview.md; then
  echo "docs/overview.md must include development/workflow.md" >&2
  exit 1
fi

echo "[docs-check] README points to workflow doc"
if ! rg -q "docs/development/workflow\\.md" README.md; then
  echo "README.md must reference docs/development/workflow.md" >&2
  exit 1
fi

echo "[docs-check] setup doc references just workflow commands"
for cmd in "just verify" "just feature" "just ship" "just push" "just pr-draft" "just sync-master"; do
  if ! rg -q "$cmd" docs/development/setup.md; then
    echo "docs/development/setup.md must include '$cmd'" >&2
    exit 1
  fi
done

echo "[docs-check] justfile recipes are documented in workflow doc"
mapfile -t recipes < <(sed -nE 's/^([a-zA-Z0-9_-]+).*/\1/p' justfile | sed '/^set$/d' | sort -u)
for recipe in "${recipes[@]}"; do
  if ! rg -q "just $recipe" docs/development/workflow.md; then
    echo "docs/development/workflow.md is missing recipe reference: just $recipe" >&2
    exit 1
  fi
done

echo "docs-check: OK"
