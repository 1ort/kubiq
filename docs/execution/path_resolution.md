# Path resolution

Путь в формате `a.b.c` разрешается в `DynamicObject.fields` как прямой ключ.

Источники ключей:

- При fetch из Kubernetes объект разворачивается в плоские `dot-path` ключи
- Для `select` в describe/json/yaml возможна обратная сборка nested-структуры

Пример:

- `metadata.name` -> `"worker-a"`
- `spec.containers.0.image` -> `"busybox:1.36"`
