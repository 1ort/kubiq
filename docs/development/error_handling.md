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
  - `ResourceNotFound`
  - `ListFailed`
  - `PaginationExceeded`
  - `PaginationStuck`
- Output layer использует typed `OutputError` (`JsonSerialize`, `YamlSerialize`)

Требование к сообщениям:

- Чётко указывать, в каком этапе произошла ошибка
- Не терять контекст внешней ошибки (`k8s error: ...`, `parse error: ...`)
