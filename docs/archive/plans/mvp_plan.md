> Status: archived  
> Archived on: 2026-02-21  
> Reason: historical completed plan for `v0.1.0` (MVP)

# MVP plan

## Этап 1 — CLI
- Парсинг аргументов
- Подключение к кластеру
- Статус: выполнено

## Этап 2 — Parser
- Грамматика
- AST
- Unit tests
- Статус: выполнено (реализовано на `nom`)

## Этап 3 — Discovery
- Разрешение ресурса
- list без фильтра
- Статус: выполнено

## Этап 4 — Engine
- Path resolver
- Expr evaluator
- Unit tests
- Статус: выполнено

## Этап 5 — Pipeline
- list -> filter -> print
- Статус: выполнено

## Этап 6 — Полировка
- Ошибки
- JSON output
- Документация
- Статус: выполнено

## Этап 7 — Select mapping
- `select` в grammar/AST
- Проекция в query plan
- default summary (`name`)
- `--describe`
- Форматы `table/json/yaml`
- Unit + e2e tests
- Статус: выполнено

## Этап 8 — Финализация MVP
- Выравнивание UX/ошибок
- Финальная ревизия документации
- Статус: выполнено

## Итог

**MVP готов.**

Release tag: `v0.1.0`

## Post-MVP updates

- Реализована pagination/batching для `list` в K8s layer (`limit/continue`).
- Реализована единая typed-иерархия ошибок (`CliError`/`K8sError`/`OutputError`) с source chain.
- Реализован safe server-side filtering pushdown (подмножество `where ==` в `fieldSelector`/`labelSelector`).
