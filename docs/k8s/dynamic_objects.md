# DynamicObject

Внутренний формат (`src/dynamic_object.rs`):

```rust
struct DynamicObject {
    fields: BTreeMap<String, serde_json::Value>,
}
```

Наполнение:

- Из Kubernetes-объекта извлекаются `metadata` и `data`
- Всё разворачивается в плоские ключи (`metadata.name`, `spec.replicas`, ...)
- Сегменты с символом `.` кодируются (`%2E`), чтобы map-ключи с точками сохранялись без искажения

Использование:

- `engine` читает поля по path
- `output` умеет:
  - summary (`name`)
  - full describe (nested)
  - select-проекцию
