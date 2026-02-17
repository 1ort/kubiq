# Roadmap to v1.0.0

Текущий статус: `v0.1.0` (MVP + post-MVP улучшения).

Цель roadmap: довести Kubiq до production-ready `v1.0.0` с предсказуемым DSL, устойчивым runtime-поведением и стабильным UX для операторских сценариев.

## Milestone 10 - Query completeness baseline (`v0.2.x`)

Цель: закрыть критичные функциональные пробелы после MVP.

Состав:
- `sort` / `order by`
- `aggregation` (минимум: `count`, `sum`, `min`, `max`, `avg`)
- Полный server-side filtering для поддерживаемых выражений `where`

Definition of Done:
- DSL/grammar/AST/engine синхронизированы
- Корректность подтверждена unit + e2e для core и CRD ресурсов
- Документация CLI и semantics обновлена

## Milestone 11 - Streaming and watch (`v0.3.x`)

Цель: добавить работу с динамическими данными и большими выборками.

Состав:
- `watch` режим в CLI
- Потоковая обработка результатов (без полной materialization в памяти)
- Базовое управление скоростью/буферами в watch-stream

Definition of Done:
- `watch` покрыт e2e-сценариями на minikube
- Потоковая модель не ломает текущие `where/select/output` гарантии
- Задокументированы ограничения и поведение при реконнектах

## Milestone 12 - Reliability hardening (`v0.4.x`)

Цель: сделать поведение в продакшн-кластерах предсказуемым.

Состав:
- typed-классификация transport/API ошибок без string эвристик
- retry/backoff/timeout policy для сетевых и transient ошибок
- discovery/resource-resolution cache
- async-first execution path (без runtime-per-request)

Definition of Done:
- Поведение ошибок стабильно и диагностично
- Есть тесты на retry/fallback ветки
- Повторные запросы к тем же ресурсам показывают снижение накладных расходов discovery

## Milestone 13 - Query language v2 (`v0.5.x`)

Цель: расширить выразительность языка без потери предсказуемости.

Состав:
- `OR`, скобки и приоритеты операций
- `IN` для множеств значений
- Расширенные string-операторы (`contains`/`starts_with`/`ends_with` или эквивалент)
- Четкие правила типовых преобразований и ошибок выражений

Definition of Done:
- Обновлены grammar + semantics + примеры
- Нет двусмысленных интерпретаций выражений
- Добавлены regression-тесты на сложные выражения и edge-кейсы

## Milestone 14 - Explainability and UX (`v0.6.x`)

Цель: улучшить дебаг и операторскую прозрачность.

Состав:
- `--explain` (план выполнения: pushdown/fallback/client-side части)
- Прозрачная индикация server-side fallback причин
- Улучшение табличного вывода (включая Unicode width)
- Единый стиль actionable diagnostics

Definition of Done:
- Пользователь видит, что выполнено на API server, а что локально
- Диагностика пригодна для copy-paste в issue/incident разбор
- UX-изменения покрыты e2e snapshot/approval тестами (или эквивалентом)

## Milestone 15 - API and integration surface (`v0.7.x`)

Цель: сделать Kubiq удобным компонентом для автоматизации.

Состав:
- Стабилизированный библиотечный API (crate-level contract)
- Machine-readable error codes
- Стандартизованные форматы JSON/YAML output для downstream tooling
- Документированный compatibility policy для minor версий

Definition of Done:
- Внешние интеграции могут пиноваться на контракт API/вывода
- Breaking changes отслеживаются и объявляются заранее

## Milestone 16 - Scale and performance (`v0.8.x`)

Цель: подтвердить эксплуатацию на больших кластерах.

Состав:
- Профилирование горячих путей (parser/evaluator/output/k8s fetch)
- Оптимизация аллокаций и копирования в pipeline
- Нагрузочные сценарии и базовые SLA (latency/throughput)

Definition of Done:
- Есть публично зафиксированные performance baselines
- Регрессии производительности детектируются в CI

## Milestone 17 - Security and policy readiness (`v0.9.x`)

Цель: подготовить релизный кандидат к enterprise-ограничениям.

Состав:
- Проверка поведения при RBAC отказах и частично доступных API group/version
- Безопасная обработка kubeconfig/context edge-кейсов
- Политика зависимостей и supply-chain hygiene

Definition of Done:
- Security/reliability checklist закрыт
- Критичные угрозы/риски либо устранены, либо задокументированы как accepted risk

## Milestone 18 - Release candidate and v1.0.0 (`v1.0.0`)

Цель: стабилизировать контракт и выпустить первую мажорную версию.

Состав:
- RC этап (`v1.0.0-rc1`, при необходимости `rc2+`)
- Freeze DSL grammar/semantics для `v1`
- Полный documentation pass
- Финальная проверка совместимости и миграционные заметки

Definition of Done:
- Все P0 пункты техдолга закрыты или явно перенесены в post-1.0 backlog
- CI/e2e green на поддерживаемой матрице окружений
- Подготовлены release notes и versioning policy для линии `v1.x`

## Cross-cutting tracks (идут параллельно всем milestones)

- Testing: unit + e2e + edge-case regression suite
- Documentation: архитектура, DSL, troubleshooting, migration notes
- Developer Experience: линтеры, форматтеры, reproducible local environment
- Product quality: управление обратной совместимостью и change communication

