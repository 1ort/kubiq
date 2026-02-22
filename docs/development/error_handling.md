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
  - `RequestTimeout`
  - `ResourceNotFound`
  - `ListFailed`
  - `ResourceResolutionStale`
  - `SelectorRejected`
  - `RetryExhausted`
  - `PaginationExceeded`
  - `PaginationStuck`
- Output layer использует typed `OutputError` (`JsonSerialize`, `YamlSerialize`)
- Реализация typed errors построена на `thiserror`
- Внутренние причины ошибок сохраняются через `source` (error chain)
- Классификация list/discovery ошибок делается typed-ветвлением по `kube::Error` (`Api` status, transport variants), без `to_string().contains(...)`
- Selector rejection классифицируется typed-правилом (`Api` 400 при активных selectors), без message-based эвристик.
- Transient API status ветки (`408`, `429`, `5xx`) относятся к retryable категории и маппятся в устойчивую typed-ветку `ApiUnreachable`/`RetryExhausted`.
- При rejected selectors используется typed fallback: повторный list без selectors + diagnostic в stderr
- При stale resource resolution (`Api` 404/410 на list после discovery) используется typed fallback: инвалидация discovery cache, однократный refresh discovery и повтор list
- Для transient ошибок применяется defaults-only retry/backoff/timeout policy; при окончательном fail выводится единая retry summary диагностика

Требование к сообщениям:

- Чётко указывать, в каком этапе произошла ошибка
- Не терять контекст внешней ошибки (`k8s error: ...`, `parse error: ...`)
- Давать actionable tips для частых операторских проблем (resource not found, API unreachable)
- Tips `DiscoveryRun`/`ListFailed`/`RetryExhausted` должны соответствовать реальной ветке причины и не конфликтовать с runtime-поведением.
