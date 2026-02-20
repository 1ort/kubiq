# Product scope (current)

Поддерживается:

- Любые ресурсы Kubernetes: core + CRD (через discovery)
- Только list-запросы
- Автоматическая pagination/batching для больших `list` (через `limit/continue`)
- Фильтрация `where`
- Операторы: `==`, `!=`
- Логика: `AND`
- Проекция полей через `select`
- Глобальные aggregation-выражения в `select`: `count(*)`, `count(path)`, `sum/min/max/avg(path)` (без `group by`)
- Сортировка `order by` (multi-key, `asc|desc`)
- Форматы вывода: `table`, `json`, `yaml`
- Режимы детализации: summary (по умолчанию), `--describe`
- Typed errors с actionable tips в CLI
- Частичный server-side filtering (safe pushdown `where ==` для `metadata.name`, `metadata.namespace`, `metadata.labels.*`)

Не поддерживается:

- watch
- join
- `group by`
- Mixing projection paths и aggregation выражений в одном `select`
- `order by` вместе с aggregation
- `--describe` вместе с aggregation
- Полный server-side filtering для всех поддерживаемых выражений `where`
