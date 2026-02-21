> Status: archived  
> Archived on: 2026-02-21  
> Reason: historical completed decomposition plan for `v0.2.0`

# v0.2.0 decomposition plan

Цель `v0.2.0`: закрыть milestone "Query completeness baseline" из `docs/plans/roadmap_v1.md`.

Статус: **completed**.

В релиз входят 3 фичи:
1. `order by` / sorting
2. `aggregation` (`count`, `sum`, `min`, `max`, `avg`)
3. Best-effort server-side filtering pushdown для поддерживаемых выражений `where`

## Delivery policy

- Каждая фича реализуется в отдельной ветке.
- После реализации: полный edge-case review, тест на каждый найденный edge-case, green test suite.

## Epic 1 - Sorting (`order by`)

Рекомендуемая ветка: `feature/sorting-order-by`

### Tasks
1. DSL/Parser
- Добавить grammar для `order by <path> [asc|desc]` (возможность нескольких ключей через `,`).
- Расширить AST/QueryPlan для хранения сортировок.
- Добавить parser unit-тесты на валидные и невалидные конструкции.

2. Engine
- Реализовать multi-key sort поверх отфильтрованных объектов.
- Определить стабильную семантику для missing/null/type-mismatch (фиксированная и документированная).
- Добавить engine unit-тесты на все комбинации (`asc/desc`, mixed types, missing fields).

3. CLI/Output
- Поддержать сортировку во всех форматах (`table/json/yaml`) без изменения select/describe поведения.
- Добавить e2e тесты с core-ресурсами и CRD.

### Definition of Done
- Сортировка детерминирована и стабильна.
- Нет регрессий для existing `where/select`.

## Epic 2 - Aggregation

Рекомендуемая ветка: `feature/aggregation-basics`

### Tasks
1. DSL/Parser
- Добавить grammar для агрегатов в select-части (например `select count(*)`, `select sum(spec.replicas)`).
- Ограничить v0.2.0 без `group by` (глобальная агрегация по result set).
- Добавить ошибки парсинга для неподдерживаемых комбинаций.

2. Engine
- Реализовать агрегаторы: `count`, `sum`, `min`, `max`, `avg`.
- Формализовать типовые правила (числовые только для `sum/avg`, поведение с null/missing).
- Вернуть агрегированный результат в совместимом формате для output layer.

3. Output
- Единый формат отображения агрегатов для `table/json/yaml`.
- Покрыть snapshot/unit тестами.

4. E2E
- Сценарии core + CRD.
- Негативные сценарии (нечисловое поле в `sum`, пустой набор данных, all-null).

### Definition of Done
- Все 5 агрегаторов работают предсказуемо.
- Ошибки агрегирования типизированы и диагностичны.

## Epic 3 - Best-effort server-side filtering pushdown

Рекомендуемая ветка: `feature/full-server-side-filtering`

### Tasks
1. Planner refactor
- Вынести `where -> ListQueryOptions` из `cli` в отдельный planner в `k8s`.
- Подготовить расширяемую модель pushdown capability.

2. Selector support expansion (technically feasible subset)
- Расширить pushdown beyond current safe subset.
- Явно разделить: полностью pushable, частично pushable, непушабельные условия.
- Гарантировать корректность через client-side post-filter (без ложноположительных результатов).

3. Fallback and diagnostics
- Typed-обработка rejected selectors (без string эвристик).
- Понятная диагностика fallback причины.

4. Testing
- Unit-тесты planner/mapping/fallback.
- E2E тесты на селекторы для core и CRD.
- Edge-кейсы: unsupported field selector, invalid label key/value, mixed pushable/non-pushable predicates.

### Definition of Done
- Реализован best-effort pushdown (только технически поддерживаемое подмножество) без потери корректности результата.
- Поведение fallback прозрачно и проверяемо тестами.

## Common technical tasks (выполняются параллельно)

1. Развязать `engine` от `parser` (P0 из `tech_debt`)
2. Убрать string-matching эвристики из K8s error mapping (P0 из `tech_debt`)
3. Исправить quoted-string/apostrophe parsing (P0 из `tech_debt`)

## Suggested implementation order

1. Epic 1 (`sorting`) - низкий риск, закрывает большой UX gap.
2. Epic 3 (`best-effort server-side filtering pushdown`) - повышает производительность и точность operator UX.
3. Epic 2 (`aggregation`) - наиболее высокая сложность семантики и output-контракта.

## Release checklist for v0.2.0

1. Все 3 epic-ветки слиты в `master`. ✅
2. Unit + e2e green. ✅
3. Обновлены: `README.md`, `docs/product/cli_spec.md`, `docs/query_language/*`, `docs/development/testing.md`, `docs/development/error_handling.md`. ✅
4. Обновлены: `docs/plans/milestones.md`, `docs/plans/roadmap_v1.md`, `docs/plans/tech_debt.md`. ✅
5. Подготовлены release notes для `v0.2.0`. ✅
