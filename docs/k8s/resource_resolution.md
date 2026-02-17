# Resource resolution

1. Найти `ApiResource` через discovery
2. Создать `Api<kube::api::DynamicObject>` через `Api::all_with(...)`
3. Выполнить paged `list` с `ListParams::limit(...)` и `continue` token
4. Преобразовать полученные объекты в внутренний `DynamicObject`

Текущая реализация выполняет list по всем namespace (all-scope), а фильтрацию делает client-side.
При пагинации есть защитные проверки: повтор токена `continue` и лимит числа страниц.
