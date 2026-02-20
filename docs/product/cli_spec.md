# CLI specification

## Формат

```bash
kubiq [--output table|json|yaml] [--describe] <resource> where <predicates> [order by <keys>] [select <paths>|<aggregations>]
```

Где:

- `<resource>`: plural-имя ресурса (`pods`, `deployments`, `widgets`)
- `<predicates>`: условия вида `<path> <op> <value>` с `AND`
- `<keys>`: ключи сортировки вида `<path> [asc|desc]` через запятую
- `<paths>`: список путей для проекции (через запятую или пробел)
- `<aggregations>`: список выражений `count(*)|count(path)|sum(path)|min(path)|max(path)|avg(path)`

## Флаги

- `--output`, `-o`: `table` (default), `json`, `yaml`
- `--describe`, `-d`: полный вывод объекта
- `--no-pushdown-warnings`: отключить предупреждения pushdown/fallback в `stderr`
- `--help`, `-h`: показать справку
- `--version`, `-V`: показать версию
- `--`: завершить разбор флагов и трактовать остаток как positional аргументы

## Правила вывода

- По умолчанию (без `select`, без `--describe`) выводится только поле `name` (`metadata.name`)
- `--describe` выводит полный nested-объект
- `select` переопределяет summary/describe и выводит только выбранные пути
- `order by` применяется после `where` и до вывода
- aggregation-`select` возвращает один агрегированный row с ключами вида `count(*)`

Ограничения aggregation:

- нельзя смешивать path-проекции и aggregation-выражения в одном `select`
- `order by` не поддерживается вместе с aggregation
- `--describe` не поддерживается вместе с aggregation

## Ошибки и диагностика

- CLI возвращает typed ошибки (`CliError`) с категориями `InvalidArgs`, `Parse`, `K8s`, `Output`
- Для частых сценариев (`resource not found`, `API unreachable`) CLI печатает actionable tips
- Для server-side filtering CLI печатает предупреждения в `stderr`, если:
  - часть предикатов не может быть pushdown'нута
  - API отверг selectors и выполнен fallback на client-side filtering
- Предупреждения pushdown можно отключить флагом `--no-pushdown-warnings`

## Примеры

```bash
kubiq pods where metadata.namespace == demo-a
kubiq pods where metadata.namespace == demo-a order by metadata.name desc
kubiq pods where metadata.namespace == demo-a select metadata.name,metadata.namespace
kubiq pods where metadata.namespace == demo-a select metadata.name order by metadata.name
kubiq -o json pods where metadata.name == worker-a select metadata
kubiq -o yaml -d pods where metadata.name == worker-a
kubiq -o json pods where metadata.namespace == demo-a select count(*)
kubiq -o json pods where metadata.namespace == demo-a select sum(metadata.generation),avg(metadata.generation)
```
