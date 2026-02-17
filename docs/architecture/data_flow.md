# Data flow

1. CLI получает `<resource> where ... [select ...]` и флаги output/detail
2. Parser строит `QueryAst`
3. Engine преобразует AST в `QueryPlan`
4. K8s layer делает discovery + list
5. Engine фильтрует объекты по `where`
6. Output применяет `select`/summary/describe
7. Output печатает в `table|json|yaml`
