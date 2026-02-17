# Resource resolution

1. Найти `ApiResource` через discovery
2. Создать `Api<kube::api::DynamicObject>` через `Api::all_with(...)`
3. Выполнить `list` с `ListParams::default()`
4. Преобразовать полученные объекты в внутренний `DynamicObject`

Текущий MVP выполняет list по всем namespace (all-scope), а фильтрацию делает client-side.
