#!/usr/bin/env bash
set -euo pipefail

contains() {
  local needle="$1"
  local file="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -Fq "$needle" "$file"
  else
    grep -Fq "$needle" "$file"
  fi
}

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
if ! contains "development/workflow.md" "docs/overview.md"; then
  echo "docs/overview.md must include development/workflow.md" >&2
  exit 1
fi

echo "[docs-check] README points to workflow doc"
if ! contains "docs/development/workflow.md" "README.md"; then
  echo "README.md must reference docs/development/workflow.md" >&2
  exit 1
fi

echo "[docs-check] setup doc references just workflow commands"
for cmd in "just verify" "just feature" "just ship" "just push" "just pr-draft" "just sync-master"; do
  if ! contains "$cmd" "docs/development/setup.md"; then
    echo "docs/development/setup.md must include '$cmd'" >&2
    exit 1
  fi
done

echo "[docs-check] justfile recipes are documented in workflow doc"
mapfile -t recipes < <(sed -nE 's/^([a-zA-Z0-9_-]+).*/\1/p' justfile | sed '/^set$/d' | sort -u)
for recipe in "${recipes[@]}"; do
  if ! contains "just $recipe" "docs/development/workflow.md"; then
    echo "docs/development/workflow.md is missing recipe reference: just $recipe" >&2
    exit 1
  fi
done

echo "docs-check: OK"
