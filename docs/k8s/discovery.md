# Discovery

Алгоритм:

1. Создать `kube::Client` из `Config::infer()`
2. Запустить `discovery::Discovery::run()`
3. Найти ресурс по plural имени (`pods`, `widgets`, ...)
4. Построить `ApiResource` через GVK + plural

Результат discovery используется для paged list-запросов к любому ресурсу (core и CRD).
