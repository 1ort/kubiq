# Examples

## Валидные

- `pods where metadata.namespace == demo-a`
- `pods where metadata.namespace == demo-a and spec.nodeName != worker-1`
- `pods where metadata.namespace == demo-a select metadata.name, metadata.namespace`
- `pods where metadata.name == worker-a select metadata`
- `widgets where spec.enabled == true select metadata.name, spec.owner`
