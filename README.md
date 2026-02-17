# Mini-KQL

Mini-KQL — CLI для выполнения SQL-подобных запросов к Kubernetes API.

Работает с:
- Pods
- Deployments
- Любым CRD

## Вывод

По умолчанию Mini-KQL отображает только имя ресурса (`name`).

- Полный вывод всех полей: `--describe` (или `-d`)
- Формат вывода: `--output table|json` (или `-o`)

Примеры:

```bash
mini-kql pods where metadata.namespace '==' demo-a
mini-kql --describe pods where metadata.namespace '==' demo-a
mini-kql -o json --describe pods where metadata.namespace '==' demo-a
```

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
