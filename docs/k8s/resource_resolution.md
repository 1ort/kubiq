# Resource resolution

1. Найти `ApiResource` через discovery
2. Создать `Api<kube::api::DynamicObject>` через `Api::all_with(...)`
3. Выполнить paged `list` с `ListParams::limit(...)` и `continue` token
4. Для безопасного подмножества `where ==` добавить server-side selectors:
   - `metadata.name`, `metadata.namespace` -> `fieldSelector`
   - `metadata.labels.*` -> `labelSelector`
5. При reject selectors от API (`BadRequest`) автоматически повторить запрос без selectors
6. Преобразовать полученные объекты в внутренний `DynamicObject`

Текущая реализация выполняет list по всем namespace (all-scope).
Фильтрация остается корректной за счет client-side evaluate для всех предикатов; server-side selectors используются как оптимизация для безопасного подмножества.
При пагинации есть защитные проверки: повтор токена `continue` и лимит числа страниц.
