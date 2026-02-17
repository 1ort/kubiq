# Examples

## Валидные

- `pods where metadata.namespace == demo-a`
- `pods where metadata.namespace == demo-a select metadata.name, metadata.namespace`
- `widgets where spec.enabled == true select metadata.name`
