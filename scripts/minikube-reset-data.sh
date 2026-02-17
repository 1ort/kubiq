#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES_FILE="$ROOT_DIR/hack/minikube/fixtures.yaml"

if ! command -v kubectl >/dev/null 2>&1; then
  echo "error: required command 'kubectl' is not installed" >&2
  exit 1
fi

echo "Re-applying fixtures to restore expected test state"
kubectl apply -f "$FIXTURES_FILE"
