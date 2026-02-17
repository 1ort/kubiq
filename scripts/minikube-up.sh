#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES_FILE="$ROOT_DIR/hack/minikube/fixtures.yaml"
NAMESPACES_FILE="$ROOT_DIR/hack/minikube/namespaces.yaml"
CRD_FILE="$ROOT_DIR/hack/minikube/crds/widgets.demo.kql.io.yaml"
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

wait_for_default_service_account() {
  local namespace="$1"
  local timeout="${2:-60}"
  local start_ts
  start_ts="$(date +%s)"

  until kubectl -n "$namespace" get serviceaccount/default >/dev/null 2>&1; do
    if (( "$(date +%s)" - start_ts >= timeout )); then
      echo "error: timed out waiting for serviceaccount/default in namespace '$namespace'" >&2
      exit 1
    fi
    sleep 1
  done
}

echo "[1/6] Starting minikube profile '$PROFILE'"
minikube start --profile "$PROFILE" --driver "$DRIVER" --kubernetes-version "$K8S_VERSION"

echo "[2/6] Updating kubectl context"
minikube update-context --profile "$PROFILE"

echo "[3/6] Installing CRDs"
kubectl apply -f "$CRD_FILE"
kubectl wait --for=condition=Established --timeout=120s crd/widgets.demo.kql.io

echo "[4/6] Creating namespaces"
kubectl apply -f "$NAMESPACES_FILE"
wait_for_default_service_account demo-a 120
wait_for_default_service_account demo-b 120

echo "[5/6] Applying test fixtures"
kubectl apply -f "$FIXTURES_FILE"

echo "[6/6] Waiting for deployments"
kubectl -n demo-a rollout status deployment/api --timeout=120s
kubectl -n demo-b rollout status deployment/web --timeout=120s

echo "Cluster is ready for Mini-KQL integration testing."
echo "Profile: $PROFILE"
