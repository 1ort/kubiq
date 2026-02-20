# Examples

## Валидные

- `pods where metadata.namespace == demo-a`
- `pods where metadata.namespace == demo-a and spec.nodeName != worker-1`
- `pods where metadata.namespace == demo-a order by metadata.name`
- `pods where metadata.namespace == demo-a order by spec.priority desc, metadata.name asc select metadata.name, metadata.namespace`
- `pods where metadata.namespace == demo-a select metadata.name, metadata.namespace order by metadata.name desc`
- `pods where metadata.name == worker-a select metadata`
- `widgets where spec.enabled == true order by spec.owner, metadata.name select metadata.name, spec.owner`
