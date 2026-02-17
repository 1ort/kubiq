# MVP scope

Поддерживается:

- Любые ресурсы Kubernetes: core + CRD (через discovery)
- Только list-запросы
- Автоматическая pagination/batching для больших `list` (через `limit/continue`)
- Фильтрация `where`
- Операторы: `==`, `!=`
- Логика: `AND`
- Проекция полей через `select`
- Форматы вывода: `table`, `json`, `yaml`
- Режимы детализации: summary (по умолчанию), `--describe`
- Typed errors с actionable tips в CLI
- Частичный server-side filtering (safe pushdown `where ==` для `metadata.name`, `metadata.namespace`, `metadata.labels.*`)

Не поддерживается:

- aggregation
- watch
- join
- sort
- Полный server-side filtering для всех поддерживаемых выражений `where`
