# Technical debt

- Нет watch
- Нет aggregation
- Нет sorting

## Done

- Pagination/batching для больших `list` (через paged requests с `limit/continue`)
- Единая typed-иерархия ошибок (`CliError`/`K8sError`/`OutputError`) с source chain (`thiserror`)
- Server-side filtering (safe pushdown подмножества `where ==` в `fieldSelector`/`labelSelector`)
