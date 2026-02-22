# Setup

## Требования

- Rust stable
- `kubectl`
- `minikube`
- Docker (если используется драйвер `docker`)
- `just` (рекомендуется для автоматизированного workflow)

Установка `just`:

```bash
cargo install just
```

## Сборка

```bash
cargo build
```

## Локальный тестовый кластер (minikube)

```bash
./scripts/minikube-up.sh
```

Скрипт поддерживает параметры окружения:

- `MINIKUBE_PROFILE` (default: `kql-dev`)
- `MINIKUBE_DRIVER` (default: `docker`)
- `MINIKUBE_K8S_VERSION` (default: `stable`)

Пример:

```bash
MINIKUBE_PROFILE=kql-ci MINIKUBE_DRIVER=docker ./scripts/minikube-up.sh
```

## Полезные проверки

```bash
cargo test -q
KUBIQ_E2E=1 cargo test --test e2e_minikube -- --nocapture
```

## Автоматизированный workflow (рекомендуется)

```bash
just bootstrap
just verify
just docs-check
just feature v0.3-discovery-cache
just ship "feat: add discovery cache"
just push
just pr-draft feat "add discovery cache" "k8s"
just sync-master feature/v0.3-discovery-cache
```

Подробности: `docs/development/workflow.md`.
