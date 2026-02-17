# Testing

## Unit tests

- parser
- path
- evaluator

## Integration tests (minikube)

- Поднять кластер:

```bash
./scripts/minikube-up.sh
```

- Проверить ресурсы:

```bash
kubectl get ns
kubectl get deploy,pod,svc,cm,secret,job -n demo-a
kubectl get deploy,pod,job -n demo-b
kubectl get crd widgets.demo.kql.io
kubectl get widgets -A
```

- Сбросить тестовые данные без удаления кластера:

```bash
./scripts/minikube-reset-data.sh
```

- Удалить кластер после тестов:

```bash
./scripts/minikube-down.sh
```

В fixtures intentionally есть разные типы сущностей (core + CRD), чтобы проверять фильтрацию Mini-KQL на разнородных объектах.

### Запуск e2e тестов

Интеграционные e2e тесты находятся в `tests/e2e_minikube.rs`.

По умолчанию они пропускаются, если не включены через env.

Запуск:

```bash
MINI_KQL_E2E=1 cargo test --test e2e_minikube -- --nocapture
```
