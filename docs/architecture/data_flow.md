# Data flow

1. CLI получает `<resource> where ... [select ...]` и флаги output/detail
2. Parser строит `QueryAst`
3. Engine преобразует AST в `QueryPlan`
4. K8s layer делает discovery + paged list (`limit/continue`)
5. Engine фильтрует объекты по `where`
6. Если запрос aggregation: Engine считает агрегаты и формирует один row
7. Иначе Engine сортирует (`order by`, если задан)
8. Output применяет `select`/summary/describe
9. Output печатает в `table|json|yaml`
