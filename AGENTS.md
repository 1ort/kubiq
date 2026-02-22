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

## Post-merge policy
- Если MR уже слит, но в feature-ветке появились новые коммиты, открывать новый MR для этих коммитов автоматически.
- После merge всегда синхронизировать локальный `master` и удалять локальную feature-ветку.

## Memory and fixation policy
- Если пользователь просит "запомнить" или "зафиксировать" правило/решение, это нужно явно записать:
  - либо в `AGENTS.md` (если правило агент-ориентированное/процессное),
  - либо в актуальную документацию проекта в `docs/` (если это проектный контракт/поведение/процесс).
- Запрос на "запомнить"/"зафиксировать" считается обязательным и задача не завершена, пока фиксация не внесена в `AGENTS.md` или `docs/` и не закоммичена.

## Documentation status sync policy
- После выполнения задачи агент обязан актуализировать её статус в релевантной документации (`docs/plans/*`, roadmap/milestones, release notes), если задача влияет на план/статусы этапов.
- Задача не считается полностью завершённой, пока статус синхронизирован в документации и изменения закоммичены.
- Если задача не относится к roadmap/планам/релизным статусам, в финальном отчёте явно указывать: `Status sync: not applicable`.

## CI triage policy
- При падении CI сначала получать статус checks, затем логи конкретного failing job.
- В отчёте пользователю всегда указывать:
  - имя check/job,
  - failing step,
  - root cause,
  - ссылку на run/job.
- После фикса обязательно:
  - запушить изменения,
  - дождаться green checks,
  - подтвердить итоговый статус и ссылку на run.
- Если `gh pr checks --watch` показывает stale/pending долго, использовать `gh run view` как источник истины по статусу run.

## Push/PR/CI sequence policy
- После завершения задачи в feature-ветке агент выполняет обязательную последовательность: `push` -> `open PR` -> `watch CI`.
- Для `push` сразу запрашивать эскалацию (`require_escalated`) и выполнять пуш без дополнительного подтверждающего шага в чате.
- После пуша агент обязан создать или обновить PR и сообщить ссылку пользователю.
- Агент обязан дождаться финального статуса checks: если есть падение, провести triage, внести фикс, запушить и повторить проверку до green.
- Задача не считается завершенной, пока PR checks не в состоянии `green` или пользователь явно не остановил ожидание.

## Automation boundaries
- Project-level scripts (`scripts/*`) и `justfile` не должны зависеть от GitHub CLI/API.
- Для GitHub operations агент может использовать отдельный skill/инструменты, но это не должно встраиваться в проектные скрипты.
- Для стандартных локальных операций агент должен использовать `just` как default entrypoint (`just-first` policy).
- Прямой вызов `scripts/*` допустим только как fallback и должен быть явно отмечен в финальном отчёте:
  - `Fallback used: <command>. Reason: <why just path was not applicable>.`

## Automation script reliability
- Скрипты в `scripts/*` должны быть portable: не требовать нестандартных утилит без fallback (например, `rg` -> fallback на `grep`).
- Для скриптов с `set -euo pipefail` избегать конструкций, падающих на пустых результатах (`ls`+glob); использовать безопасные проверки (`find`, `test`, явные условия).
- Изменения в automation-скриптах должны сопровождаться smoke-проверками локально и в CI.

## Canonical just mapping
- `just verify` -> `./scripts/verify.sh`
- `just automation-smoke` -> `./scripts/automation-smoke.sh`
- `just hygiene-check` -> `./scripts/hygiene-smoke.sh`
- `just docs-check` -> `./scripts/docs-check.sh`
- `just feature <name>` -> `./scripts/git/feature.sh`
- `just ship <msg>` -> `./scripts/git/ship.sh`
- `just push` -> `./scripts/git/push.sh`
- `just sync-master [branch]` -> `./scripts/git/sync_master.sh`
- `just pr-draft <type> <title> [scope]` -> `./scripts/pr/generate_pr.sh`
