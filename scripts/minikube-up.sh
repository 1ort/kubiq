#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES_FILE="$ROOT_DIR/hack/minikube/fixtures.yaml"
PROFILE="${MINIKUBE_PROFILE:-kql-dev}"
DRIVER="${MINIKUBE_DRIVER:-docker}"
K8S_VERSION="${MINIKUBE_K8S_VERSION:-stable}"

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: required command '$cmd' is not installed" >&2
    exit 1
  fi
}

require_cmd minikube
require_cmd kubectl

echo "[1/4] Starting minikube profile '$PROFILE'"
minikube start --profile "$PROFILE" --driver "$DRIVER" --kubernetes-version "$K8S_VERSION"

echo "[2/4] Updating kubectl context"
minikube update-context --profile "$PROFILE"

echo "[3/4] Applying test fixtures"
kubectl apply -f "$FIXTURES_FILE"

echo "[4/4] Waiting for deployments"
kubectl -n demo-a rollout status deployment/api --timeout=120s
kubectl -n demo-b rollout status deployment/web --timeout=120s

echo "Cluster is ready for Mini-KQL integration testing."
echo "Profile: $PROFILE"
