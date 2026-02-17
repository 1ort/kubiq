# Design principles

1. DynamicObject — единый формат данных
2. Parser не знает про Kubernetes
3. Engine не знает про CLI
4. evaluate() — чистая функция
5. Никакой бизнес-логики в main.rs
