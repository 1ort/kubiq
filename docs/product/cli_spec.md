# CLI specification

## Формат

```bash
kubiq [--output table|json|yaml] [--describe] <resource> where <predicates> [select <paths>]
```

Где:

- `<resource>`: plural-имя ресурса (`pods`, `deployments`, `widgets`)
- `<predicates>`: условия вида `<path> <op> <value>` с `AND`
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

## Примеры

```bash
kubiq pods where metadata.namespace == demo-a
kubiq pods where metadata.namespace == demo-a select metadata.name,metadata.namespace
kubiq -o json pods where metadata.name == worker-a select metadata
kubiq -o yaml -d pods where metadata.name == worker-a
```
