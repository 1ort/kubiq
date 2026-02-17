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
1. Определить затронутые слои и обновить их при необходимости (grammar, AST, evaluator, k8s, output).
2. Реализовать фичу в коде.
3. Полностью пройти по затронутому коду и найти все edge-кейсы.
4. Для каждого edge-case добавить отдельный тест.
5. Добавить/обновить остальные тесты на фичу.
6. Убедиться, что все тесты (включая новые edge-case тесты) проходят.
7. Запустить линтеры и форматтеры.
8. Обновить docs.
9. Закоммитить изменения.

## Directory roles
- cli/ — аргументы и запуск
- parser/ — DSL
- engine/ — выполнение выражений
- k8s/ — работа с API
- output/ — форматирование результата

## Commit policy
- Коммитить изменения сразу после выполнения задачи, без отдельного запроса пользователя.
- Каждую отдельную фичу разрабатывать в отдельной git-ветке.
