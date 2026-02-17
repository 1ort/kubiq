# MVP plan

## Этап 1 — CLI
- Парсинг аргументов
- Подключение к кластеру

## Этап 2 — Parser
- Грамматика
- AST
- Unit tests

## Этап 3 — Discovery
- Разрешение ресурса
- list без фильтра

## Этап 4 — Engine
- Path resolver
- Expr evaluator
- Unit tests

## Этап 5 — Pipeline
- list → filter → print

## Этап 6 — Полировка
- Ошибки
- JSON output
- Документация

## Этап 7 — Select mapping
- Добавить `select` в grammar
- Расширить AST для маппинга полей
- Добавить проекцию в query plan (`where` + `select`)
- Реализовать вывод только выбранных полей
- По умолчанию выводить только `metadata.name`
- Добавить `--describe` для полного вывода ресурса
- Unit и интеграционные тесты
