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

## Select / output projection

- `select` оставляет только указанные пути
- Если выбран родительский путь (например `metadata`), в `json|yaml` восстанавливается nested-объект из `metadata.*`
- Отсутствующий выбранный путь -> `null` (`json|yaml`) или `-` (`table`)
- `select` имеет приоритет над default summary и `--describe`
