# Design principles

1. `DynamicObject` — единый внутренний формат ресурса
2. Parser не зависит от Kubernetes
3. Engine не зависит от CLI/K8s
4. `evaluate()` — чистая функция
5. В `main.rs` только запуск приложения
