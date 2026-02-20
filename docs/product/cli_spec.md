# CLI specification

## Формат

```bash
kubiq [--output table|json|yaml] [--describe] <resource> where <predicates> [order by <keys>] [select <paths>]
```

Где:

- `<resource>`: plural-имя ресурса (`pods`, `deployments`, `widgets`)
- `<predicates>`: условия вида `<path> <op> <value>` с `AND`
- `<keys>`: ключи сортировки вида `<path> [asc|desc]` через запятую
- `<paths>`: список путей для проекции (через запятую или пробел)

## Флаги

- `--output`, `-o`: `table` (default), `json`, `yaml`
- `--describe`, `-d`: полный вывод объекта
- `--help`, `-h`: показать справку
- `--version`, `-V`: показать версию
- `--`: завершить разбор флагов и трактовать остаток как positional аргументы

## Правила вывода

- По умолчанию (без `select`, без `--describe`) выводится только поле `name` (`metadata.name`)
- `--describe` выводит полный nested-объект
- `select` переопределяет summary/describe и выводит только выбранные пути
- `order by` применяется после `where` и до вывода

## Ошибки и диагностика

- CLI возвращает typed ошибки (`CliError`) с категориями `InvalidArgs`, `Parse`, `K8s`, `Output`
- Для частых сценариев (`resource not found`, `API unreachable`) CLI печатает actionable tips

## Примеры

```bash
kubiq pods where metadata.namespace == demo-a
kubiq pods where metadata.namespace == demo-a order by metadata.name desc
kubiq pods where metadata.namespace == demo-a select metadata.name,metadata.namespace
kubiq pods where metadata.namespace == demo-a select metadata.name order by metadata.name
kubiq -o json pods where metadata.name == worker-a select metadata
kubiq -o yaml -d pods where metadata.name == worker-a
```
