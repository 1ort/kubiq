# AGENTS.md

## Project goal
Kubiq — CLI-инструмент для выполнения SQL-подобных запросов к Kubernetes API поверх любых ресурсов (core и CRD).

## MVP scope
- Любые ресурсы (core + CRD)
- Только `list`
- Только `where`
- Операторы: `==`, `!=`
- Логика: `AND`
- Без aggregation
- Без watch

## Execution pipeline
CLI → parse → AST → query plan → fetch → evaluate → output

## Source of truth
- Архитектура: `ARCHITECTURE.md`
- DSL: `docs/query_language/`
- План разработки: `docs/plans/mvp_plan.md`

## Core invariants
1. Engine не зависит от Kubernetes.
2. Parser не зависит от engine.
3. evaluate() — чистая функция.
4. DynamicObject — единый формат ресурса.

## How to add a feature
1. Обновить grammar.
2. Обновить AST.
3. Обновить evaluator.
4. Добавить тесты.
5. Убедиться, что тесты проходят.
6. Запустить линтеры и форматтеры.
5. Обновить docs.
6. Закоммитить изменения.

## Directory roles
- cli/ — аргументы и запуск
- parser/ — DSL
- engine/ — выполнение выражений
- k8s/ — работа с API
- output/ — форматирование результата

## Commit policy
- Коммитить изменения сразу после выполнения задачи, без отдельного запроса пользователя.
- Каждую отдельную фичу разрабатывать в отдельной git-ветке.
