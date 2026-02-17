# Technical debt

- Нет server-side filtering
- Нет watch
- Нет aggregation
- Нет sorting
- Ошибки пока на строковых типах (нет единой иерархии typed errors)

## Done

- Pagination/batching для больших `list` (через paged requests с `limit/continue`)
