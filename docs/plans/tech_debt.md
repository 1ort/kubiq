# Technical debt

## Top product gaps

1. Sorting (`order by`) отсутствует
2. Watch-режим отсутствует
3. Aggregation отсутствует
4. Полный server-side filtering отсутствует (сейчас только safe subset)

## Refactoring and quality backlog

### P0 (высокий приоритет)

1. Развязать `engine` от `parser` типов (нарушение core invariant)
- Где: `src/engine/mod.rs`
- Проблема: `QueryPlan` хранит `parser::Predicate` и `parser::Operator`, из-за чего engine зависит от parser.
- Что сделать: ввести engine-owned типы (`EnginePredicate`, `EngineOperator`), конвертировать AST -> QueryPlan на boundary.
- Критерий готовности: `engine` не импортирует `crate::parser`; тесты `engine` используют только engine-типы.

2. Убрать строковые эвристики классификации ошибок K8s
- Где: `src/k8s/mod.rs` (`is_api_unreachable_error_message`, `should_retry_without_selectors`)
- Проблема: проверка по `contains(...)` хрупкая и чувствительна к формату текста клиента/апи-сервера.
- Что сделать: перейти на typed-ветвление по `kube::Error`/HTTP status (`BadRequest`, transport/connectivity), вынести в отдельный mapper.
- Критерий готовности: нет бизнес-решений через `error.to_string().contains(...)`; есть unit-тесты на typed mapper.

3. Исправить обработку значений с `'` в аргументах CLI
- Где: `src/parser/mod.rs` (`normalize_arg`, `quoted_string_value`)
- Проблема: аргументы с апострофом ломают синтаксис, escape-последовательности не поддерживаются.
- Что сделать: добавить корректное экранирование/разбор quoted string (например `\'`) и тесты на такие входы.
- Критерий готовности: запросы вида `where metadata.name == O'Reilly` корректно парсятся в string literal.

### P1 (средний приоритет)

1. Уменьшить связность CLI и K8s pushdown логики
- Где: `src/cli/mod.rs` (`build_list_query_options` и selector validators)
- Проблема: логика маппинга `where -> selectors` живет в CLI, хотя относится к K8s fetch strategy.
- Что сделать: вынести pushdown planner в `k8s` (или отдельный модуль planner), в CLI оставить только orchestration.
- Критерий готовности: CLI не знает детали `fieldSelector`/`labelSelector` сборки.

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
- Server-side filtering (safe pushdown подмножества `where ==` в `fieldSelector`/`labelSelector`)
