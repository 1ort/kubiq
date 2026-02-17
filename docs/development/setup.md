# Setup

## Требования

- Rust stable
- `kubectl`
- `minikube`
- Docker (если используется драйвер `docker`)

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
