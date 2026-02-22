# Path resolution

Путь в формате `a.b.c` разрешается в `DynamicObject.fields` как прямой ключ.

Для внутренних ключей `fields` используется segment-level encoding:
- `.` внутри map-key сегмента кодируется как `%2E`
- `%` внутри сегмента кодируется как `%25`

Это позволяет сохранить ключи вида `kubectl.kubernetes.io/...` без расщепления на path-сегменты.

Источники ключей:

- При fetch из Kubernetes объект разворачивается в плоские `dot-path` ключи
- Для `select` в describe/json/yaml возможна обратная сборка nested-структуры

Пример:

- `metadata.name` -> `"worker-a"`
- `spec.containers.0.image` -> `"busybox:1.36"`
