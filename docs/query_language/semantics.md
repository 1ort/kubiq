# Semantics

## Where / evaluation

- Отсутствующее поле -> `false` для `==` и `!=`
- Несовпадение типов -> `false` для `==` и `!=`
- `null` в сравнении -> `false`
- `AND` вычисляется как `all()` (короткое замыкание)

## Value typing

Значение в предикате интерпретируется как:

1. `true|false` -> boolean
2. целое/вещественное -> number
3. остальное -> string

Строковый литерал в `'...'` всегда string.

## Order by / sorting

- Сортировка применяется после `where` и до `select`/рендера.
- Поддерживается multi-key сортировка: `order by a, b desc, c asc`.
- Направление по умолчанию: `asc`.
- Политика `null`/missing: SQL-style
  - `asc`: `null` и missing идут первыми
  - `desc`: `null` и missing идут последними
- Mixed types сравниваются по фиксированному приоритету типов:
  - `bool < number < string < other(json)` для `asc`
  - для `desc` порядок инвертируется
- Для полностью равных ключей сохраняется исходный порядок (stable sort).

## Select / output projection

- `select` оставляет только указанные пути
- Если выбран родительский путь (например `metadata`), в `json|yaml` восстанавливается nested-объект из `metadata.*`
- Отсутствующий выбранный путь -> `null` (`json|yaml`) или `-` (`table`)
- `select` имеет приоритет над default summary и `--describe`

## Aggregation

- Aggregation задается в `select`: `count(*)`, `count(path)`, `sum(path)`, `min(path)`, `max(path)`, `avg(path)`.
- В одном `select` нельзя смешивать path-проекции и агрегации.
- Aggregation и `order by` не комбинируются.
- `--describe` не поддерживается для aggregation-запросов.
- Результат aggregation — один row (`items: 1`) с ключами вида `count(*)`, `sum(spec.replicas)`.

Политика `null`/missing и типов:

- `count(*)`: считает все строки после `where`.
- `count(path)`: считает только non-null существующие значения.
- `sum(path)` / `avg(path)`: принимают только `number` (non-null). Иначе ошибка.
- `min(path)` / `max(path)`: принимают homogeneous тип (`bool` или `number` или `string`). Mixed types -> ошибка.

Пустой набор:

- `count(*) = 0`
- `count(path) = 0`
- `sum(path) = 0`
- `avg/min/max = null`
