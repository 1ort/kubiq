# Documentation policy

## Purpose

Этот документ фиксирует жизненный цикл документации и определяет, где хранится актуальная спецификация Kubiq.

## Source of truth

- Архитектура: `docs/architecture/`
- DSL (grammar/semantics/examples): `docs/query_language/`
- Публичный CLI-контракт: `docs/product/cli_spec.md`
- Текущий scope: `docs/product/scope.md`
- Актуальная навигация: `docs/overview.md`

`ARCHITECTURE.md` используется как краткий обзор и не должен расходиться с `docs/architecture/`.

## Document statuses

- `active`: документ описывает текущее поведение/контракт.
- `completed`: документ описывает уже завершенный план, но еще не архивирован.
- `archived`: исторический документ; не должен использоваться как текущий source of truth.

## Lifecycle rules

1. После релиза completed-планы переносятся в `docs/archive/`.
2. `docs/overview.md` включает только active-документы в основном списке чтения.
3. Archived-документы перечисляются отдельным разделом `Historical archive`.
4. При изменении CLI/DSL/архитектуры обязательно обновляются соответствующие active-документы.
5. Если документ конфликтует с active source of truth, он должен быть исправлен или архивирован в том же PR.

## Release-time checklist (docs)

1. Проверить `README.md` и `docs/product/cli_spec.md` на соответствие текущему baseline.
2. Проверить `docs/architecture/` и `ARCHITECTURE.md` на отсутствие противоречий.
3. Обновить `docs/overview.md` (active + archive разделы).
4. Перенести завершенные планы в `docs/archive/`.
5. Проверить markdown-ссылки после перемещений.
