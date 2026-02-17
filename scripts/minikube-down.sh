#!/usr/bin/env bash
set -euo pipefail

PROFILE="${MINIKUBE_PROFILE:-kql-dev}"

if ! command -v minikube >/dev/null 2>&1; then
  echo "error: required command 'minikube' is not installed" >&2
  exit 1
fi

echo "Deleting minikube profile '$PROFILE'"
minikube delete --profile "$PROFILE"
