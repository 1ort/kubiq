# v0.3.0 decomposition plan

Цель `v0.3.0`: закрыть Milestone 11 "Reliability hardening" из `docs/plans/roadmap_v1.md` и закрыть P1 technical debt из `docs/plans/tech_debt.md`.

Статус: **in progress** (`Epic 1/2/3/5` completed; `Epic 4` planned).

В релиз входят 5 направлений:
1. Async-first execution path (без runtime-per-request)
2. Retry/backoff/timeout policy (defaults-only)
3. Discovery/resource-resolution cache
4. Typed error hardening audit
5. Unified path utilities + dotted keys fix

## Delivery policy

- Каждое направление реализуется в отдельной ветке.
- После реализации: полный edge-case review, тест на каждый найденный edge-case, green test suite.
- Для `v0.3.0` не добавляются новые DSL конструкции и CLI knobs для retry/timeout.

## Epic 1 - Async-first execution path

Рекомендуемая ветка: `feature/v0.3-async-first-runtime`
Статус: **completed**.

Результат:
- Добавлен async-first API в K8s слое: `list_async(...)`.
- Основной CLI pipeline переведен на async execution.
- Runtime инициализируется один раз на процесс в бинарнике (process-wide Tokio runtime через `Runtime::new()`), без runtime-per-request в hot path `list`.
- Sync entrypoints сохранены как thin compatibility wrappers.

### Tasks
1. API and runtime model
- Добавить async-first API в K8s слое: `list_async(...)`.
- Перевести основной CLI pipeline на async execution.
- Инициализировать runtime один раз на процесс в бинарнике (допустимы `#[tokio::main]` или эквивалентный process-wide runtime).
- Убрать `Runtime::new()` из hot path list-запросов.

2. Compatibility
- Сохранить sync entrypoint только как thin compatibility wrapper (если нужен для внешнего кода).
- Убедиться, что поведение CLI (результаты и ошибки) не меняется.

3. Validation
- Добавить unit/integration тесты на отсутствие runtime-per-request поведения.
- Проверить отсутствие регрессий для `where/select/order by/aggregation` (текущее состояние: green unit suite).

### Definition of Done
- Runtime инициализируется один раз на процесс.
- Нет поведенческих регрессий в CLI pipeline.

## Epic 2 - Retry/backoff/timeout policy

Рекомендуемая ветка: `feature/v0.3-retry-timeout-policy`
Статус: **completed**.

Результат:
- Добавлена встроенная defaults-only policy: bounded retries + exponential backoff + request timeout.
- Retry применяется только для transient веток (`transport`, timeout, `429`, `5xx`), non-retryable ошибки завершаются немедленно.
- Добавлена итоговая retry-диагностика в stderr (final summary) без шумного по-попыточного логирования.
- Поведение покрыто unit-тестами на retry branches, retry cap, timeout path и non-retryable scenarios.

### Tasks
1. Policy
- Ввести встроенную default policy (без новых CLI/env параметров):
  - bounded retries
  - exponential backoff
  - request timeout
- Типизировать retryable/non-retryable ветки по `kube::Error` variants и API status codes.

2. Error behavior
- Retry только для transient ошибок (transport, timeout, 429, 5xx).
- Немедленный fail для не-retryable ошибок (валидационные 4xx, `ResourceNotFound` и др.).
- Сохранить typed error surface и source chain.

3. Diagnostics
- Добавить диагностику retry/final-failure в stderr в едином стиле с текущими pushdown diagnostics.

4. Testing
- Unit-тесты на retry branches, timeout path, retry cap, backoff guardrails.
- Негативные тесты на non-retryable scenarios.

### Definition of Done
- Есть тесты на retry/fallback ветки.
- Поведение ошибок детерминировано и диагностично.

## Epic 3 - Discovery/resource-resolution cache

Рекомендуемая ветка: `feature/v0.3-discovery-cache`
Статус: **completed**.

Результат:
- Добавлен in-memory discovery cache (`resource -> ApiResource`) с фиксированным TTL (`v0.3` defaults-only).
- Повторные запросы к тем же ресурсам в рамках TTL используют cache-hit вместо полного discovery.
- Добавлена инвалидация cache и однократный retry с fresh discovery при typed stale-resolution ошибках list (`Api` 404/410).
- Добавлены unit-тесты на hit/miss, expiry, invalidation и stale-classification.

### Tasks
1. Cache implementation
- Добавить in-memory cache для `resource -> ApiResource`.
- Включить TTL (фиксированная default-константа для `v0.3.0`).
- Добавить invalidation при ошибках, указывающих на потенциально stale resolution.

2. Integration
- Интегрировать cache в `resolve_api_resource`.
- Гарантировать, что повторные запросы к одному ресурсу не запускают full discovery.

3. Testing
- Unit-тесты на hit/miss, expiry, invalidation.
- Integration/e2e сценарии на повторные запросы.

### Definition of Done
- Повторные запросы к тем же ресурсам снижают discovery overhead.
- Корректность resource resolution не деградирует.

## Epic 4 - Typed error hardening audit

Рекомендуемая ветка: `feature/v0.3-error-hardening-audit`

### Tasks
1. Audit
- Провести аудит всех веток K8s transport/API ошибок (discovery/list/retry/timeout/pagination).
- Удалить остаточные string-based эвристики там, где можно заменить typed-классификацией.

2. Error model
- При необходимости расширить/уточнить `K8sError` варианты без ломки CLI UX.
- Проверить соответствие CLI tips фактическим категориям ошибок.

3. Testing
- Unit-тесты на корректный mapping `kube::Error -> K8sError`.
- Тесты на стабильный текст actionable diagnostics.

### Definition of Done
- Typed error mapping предсказуем и устойчив.
- Ошибки остаются операбельно диагностируемыми.

## Epic 5 - Unified path utilities + dotted keys fix (P1 debt)

Рекомендуемая ветка: `feature/v0.3-path-utils-dotted-keys`
Статус: **completed**.

Результат:
- Flatten/unflatten/select-path логика сведена в единый модуль path utilities и подключена в `k8s` + `output`.
- Добавлено segment-level percent-encoding для dotted map keys (`.`/`%`) с обратимым decode.
- Roundtrip для `metadata.annotations`/`metadata.labels` с dotted keys сохраняет исходные ключи без искажения.
- Добавлены unit/e2e тесты на parent-path select/describe с ключами вида `kubectl.kubernetes.io/...`.

### Tasks
1. Refactor path logic
- Вынести flatten/unflatten/select-path логику в единый модуль path utilities.
- Подключить этот модуль в `k8s` и `output`, убрать дубли.

2. Dotted key correctness
- Добавить экранирование/кодирование path segments для map-ключей с `.`.
- Гарантировать корректный roundtrip для `metadata.annotations`/`metadata.labels` с dotted keys.

3. Testing
- Unit-тесты на roundtrip nested structures.
- Тесты на select/describe для parent path при наличии dotted keys.
- E2E сценарии на реальные аннотации вида `kubectl.kubernetes.io/...`.

### Definition of Done
- Ключи с `.` не искажаются в select/describe.
- Семантика путей едина в fetch/output слоях.

## Common technical tasks (cross-cutting)

1. Обновить документацию:
- `ARCHITECTURE.md`
- `docs/architecture/*` (data flow/module structure при необходимости)
- `docs/development/error_handling.md`
- `docs/development/testing.md`
- `docs/plans/milestones.md`
- `docs/plans/roadmap_v1.md` (статус milestone после релиза)

2. Подготовить release notes:
- `docs/releases/v0.3.0.md`

3. Валидация качества:
- `cargo test`
- e2e на minikube (`KUBIQ_E2E=1 cargo test --test e2e_minikube -- --nocapture`)

## Suggested implementation order

1. Epic 1 (`async-first execution`)
2. Epic 3 (`discovery cache`)
3. Epic 2 (`retry/backoff/timeout`)
4. Epic 4 (`typed error hardening audit`)
5. Epic 5 (`path utilities + dotted keys`)

## Acceptance criteria for v0.3.0

1. Milestone 11 из roadmap закрыт фактически и документально.
2. P1 debt задачи по async runtime, discovery cache, path unification и dotted keys закрыты.
3. Unit + e2e green.
4. Результаты CLI для текущего query scope (`v0.2.0`) не имеют функциональных регрессий.
5. Документация и release notes синхронизированы с реализацией.

## Explicit assumptions and defaults

1. В `v0.3.0` нет изменений DSL grammar/semantics.
2. Retry/timeout policy не конфигурируется пользователем.
3. Discovery cache локальный in-memory, без персистентности.
4. TTL и retry-параметры фиксированы в коде и могут быть вынесены в настройки в последующих релизах.
