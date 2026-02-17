# Mini-KQL

Mini-KQL — CLI для выполнения SQL-подобных запросов к Kubernetes API.

Работает с:
- Pods
- Deployments
- Любым CRD

## Быстрый тестовый кластер (minikube / "minicube")

Для локальной интеграционной проверки можно поднять кластер и заполнить его тестовыми сущностями:

```bash
./scripts/minikube-up.sh
```

В кластере будут созданы:
- Namespaces: `demo-a`, `demo-b`
- Core ресурсы: `Pod`, `Deployment`, `Service`, `ConfigMap`, `Secret`, `Job`
- CRD: `widgets.demo.kql.io`
- CR: `Widget` в двух namespace

Перезаполнить тестовые данные:

```bash
./scripts/minikube-reset-data.sh
```

Удалить кластер:

```bash
./scripts/minikube-down.sh
```
