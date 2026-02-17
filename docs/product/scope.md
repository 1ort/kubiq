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

Не поддерживается:

- aggregation
- watch
- join
- sort
- server-side filtering (пока всё фильтруется client-side)
