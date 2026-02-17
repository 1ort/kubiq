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

Использование:

- `engine` читает поля по path
- `output` умеет:
  - summary (`name`)
  - full describe (nested)
  - select-проекцию
