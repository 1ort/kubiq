# Error handling

Текущее состояние:

- CLI использует `CliError` с категориями:
  - `InvalidArgs`
  - `Parse`
  - `K8s`
  - `Output`
- K8s layer использует typed `K8sError`:
  - `EmptyResourceName`
  - `RuntimeInit`
  - `ConfigInfer`
  - `ClientBuild`
  - `DiscoveryRun`
  - `ApiUnreachable`
  - `ResourceNotFound`
  - `ListFailed`
  - `SelectorRejected`
  - `PaginationExceeded`
  - `PaginationStuck`
- Output layer использует typed `OutputError` (`JsonSerialize`, `YamlSerialize`)
- Реализация typed errors построена на `thiserror`
- Внутренние причины ошибок сохраняются через `source` (error chain)
- Классификация list/discovery ошибок делается typed-ветвлением по `kube::Error` (`Api` status, transport variants), без `to_string().contains(...)`
- При rejected selectors используется typed fallback: повторный list без selectors + diagnostic в stderr

Требование к сообщениям:

- Чётко указывать, в каком этапе произошла ошибка
- Не терять контекст внешней ошибки (`k8s error: ...`, `parse error: ...`)
- Давать actionable tips для частых операторских проблем (resource not found, API unreachable)
