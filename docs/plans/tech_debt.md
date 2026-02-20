# Technical debt

## Top product gaps

1. Watch-режим отсутствует
2. Aggregation отсутствует

## Refactoring and quality backlog

### P0 (высокий приоритет)

Открытых P0 задач нет.

### P1 (средний приоритет)

2. Убрать создание отдельного Tokio runtime на каждый `list`
- Где: `src/k8s/mod.rs` (`Runtime::new`, `block_on`)
- Проблема: избыточные накладные расходы и ухудшение composability библиотечного API.
- Что сделать: сделать async путь первичным (`pub async fn list_async`), в бинарнике использовать `#[tokio::main]`.
- Критерий готовности: runtime инициализируется один раз на процесс; sync-wrapper (если нужен) тонкий и изолированный.

3. Добавить кэш discovery/разрешения ресурса
- Где: `src/k8s/mod.rs` (`resolve_api_resource`)
- Проблема: discovery запускается на каждый запрос, что увеличивает latency и нагрузку на API server.
- Что сделать: локальный cache (`resource -> ApiResource`) с инвалидацией по TTL/ошибке.
- Критерий готовности: повторные запросы к одному ресурсу не вызывают полный discovery каждый раз.

4. Свести flatten/unflatten path-логику в единый модуль
- Где: `src/k8s/mod.rs` (`flatten_value`), `src/output/mod.rs` (`insert_nested_value`)
- Проблема: дублирование и риск расхождения семантики путей/массивов.
- Что сделать: вынести path utilities в отдельный модуль и использовать в fetch/output.
- Критерий готовности: единый набор тестов покрывает flatten + reconstruction roundtrip.

### P2 (низкий приоритет, но полезно закрыть)

1. Улучшить устойчивость e2e-запуска
- Где: `tests/e2e_minikube.rs`
- Проблема: тесты тихо пропускаются по env/готовности кластера; нет явной диагностики причин skip.
- Что сделать: добавить информативный `eprintln!` при skip и отдельный smoke-check fixture readiness.
- Критерий готовности: при пропуске видно точную причину; flaky-сценарии диагностируются быстрее.

2. Табличный рендер для широких/Unicode значений
- Где: `src/output/mod.rs` (`compute_widths`, `format_row`)
- Проблема: ширина считается через `len()`, визуально ломается на wide/unicode символах.
- Что сделать: использовать display-width расчет (например `unicode-width`) и тест-кейсы на wide chars.
- Критерий готовности: таблица выровнена для ASCII и Unicode кейсов.

## Done

- Pagination/batching для больших `list` (через paged requests с `limit/continue`)
- Единая typed-иерархия ошибок (`CliError`/`K8sError`/`OutputError`) с source chain (`thiserror`)
- Server-side filtering planner + pushdown подмножества `where` (`==`/`!=` для metadata/labels) с typed fallback diagnostics
- Typed mapper ошибок K8s list/discovery без string-эвристик
- Pushdown planner вынесен из CLI в `k8s::planner`
- Исправлен парсинг string-литералов с `'` и escape-последовательностями в CLI query args/parser
- `engine` отвязан от `parser` типов: `engine::QueryPlan` хранит engine-owned типы, AST конвертируется на CLI boundary
