# Milestones

1. Parser работает (`where`, `and`, typed values, `select`) — ✅
2. Discovery работает для core + CRD — ✅
3. Engine корректно фильтрует объекты — ✅
4. CLI поддерживает `table|json|yaml`, `--describe`, `select` — ✅
5. Есть e2e покрытие на minikube — ✅
6. MVP готов — ✅
7. Pagination/batching для `list` реализованы — ✅
8. Typed error hierarchy (`CliError`/`K8sError`/`OutputError`) реализована — ✅
9. Server-side filtering (safe pushdown subset) реализован — ✅
10. Query completeness baseline (`sort`, `aggregation`, best-effort server-side filtering pushdown) — ✅

## Next milestones toward v1.0.0

11. Streaming and watch
12. Reliability hardening (typed error mapping, retry/backoff, async-first, discovery cache)
13. Query language v2 (`OR`, parentheses, `IN`, extended string operators)
14. Explainability and UX (`--explain`, fallback transparency, output UX polish)
15. API and integration surface stabilization
16. Scale and performance baselines
17. Security and policy readiness
18. Release candidate and `v1.0.0`

Подробная дорожная карта: `docs/plans/roadmap_v1.md`
Декомпозиция `v0.2.0`: `docs/plans/v0_2_0_plan.md`
