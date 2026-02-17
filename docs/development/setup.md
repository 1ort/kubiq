# Setup

## Требования

- Rust stable
- Доступ к Kubernetes cluster
- `kubectl`
- `minikube`
- Docker (если используете драйвер `docker`)

## Сборка

```bash
cargo build
```

## Локальный тестовый кластер (minikube)

Поднять локальный кластер и загрузить тестовые сущности:

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
