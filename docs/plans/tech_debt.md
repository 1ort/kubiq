# Technical debt

- Нет server-side filtering
- Нет watch
- Нет aggregation
- Нет sorting

## Done

- Pagination/batching для больших `list` (через paged requests с `limit/continue`)
- Единая typed-иерархия ошибок (`CliError`/`K8sError`/`OutputError`) с source chain (`thiserror`)
