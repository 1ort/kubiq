# Coding guidelines

1. Не писать бизнес-логику в `main.rs`
2. Parser не зависит от engine/k8s/output
3. Engine не зависит от k8s/cli
4. Использовать `Result`, избегать panic в production-коде
5. Добавлять unit-тесты на каждую новую ветку grammar/evaluation/output
6. Для пользовательских сценариев добавлять e2e в `tests/e2e_minikube.rs`
