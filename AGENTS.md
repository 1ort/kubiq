# AGENTS.md

## Project goal
Kubiq — CLI-инструмент для выполнения SQL-подобных запросов к Kubernetes API поверх любых ресурсов (core и CRD).

## Current scope (`v0.2.0`)
- Любые ресурсы (core + CRD)
- Только `list`
- `where` + `select` + `order by`
- Операторы: `==`, `!=`
- Логика: `AND`
- Глобальная aggregation в `select`: `count`, `sum`, `min`, `max`, `avg` (без `group by`)
- Best-effort server-side filtering pushdown для поддерживаемого подмножества `where`
- Форматы вывода: `table`, `json`, `yaml`; режимы: summary, `--describe`
- Без watch

## Execution pipeline
CLI → parse → AST → query plan → fetch → evaluate → (aggregate | sort) → project/output

## Source of truth
- Архитектура (детально): `docs/architecture/`
- Архитектура (кратко): `ARCHITECTURE.md`
- DSL: `docs/query_language/`
- CLI контракт: `docs/product/cli_spec.md`
- Актуальная карта документации: `docs/overview.md`
- Дорожная карта: `docs/plans/roadmap_v1.md`

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

## Run kubiq correctly (for agents)
- Перед запуском `kubiq` обязательно очищать proxy env vars, иначе возможны ложные ошибки `kubernetes api is unreachable`.
- Рекомендуемый шаблон запуска из репозитория:
  - `env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy cargo run -- <query args>`
- Если бинарь уже собран, можно быстрее:
  - `env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy target/debug/kubiq <query args>`
- Для быстрой проверки доступа к кластеру использовать:
  - `kubectl config current-context`
  - `kubectl get ns`

## Commit policy
- Коммитить изменения сразу после выполнения задачи, без отдельного запроса пользователя.
- Каждую отдельную фичу разрабатывать в отдельной git-ветке.
- Для `git push` сразу запрашивать эскалацию (`require_escalated`), без предварительной попытки в sandbox.
- После пуша feature-ветки обязательно подготовить название и описание MR (PR).
- После пуша feature-ветки, если технически возможно, открывать MR (PR) автоматически без дополнительного запроса.
- Текст названия и описания MR (PR) всегда писать на английском языке.
- Если пользователь явно просит реализовать несколько независимых пунктов, каждый пункт фиксировать отдельным коммитом.

## Memory and fixation policy
- Если пользователь просит "запомнить" или "зафиксировать" правило/решение, это нужно явно записать:
  - либо в `AGENTS.md` (если правило агент-ориентированное/процессное),
  - либо в актуальную документацию проекта в `docs/` (если это проектный контракт/поведение/процесс).
