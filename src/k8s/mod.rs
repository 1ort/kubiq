use std::collections::BTreeMap;

use kube::{
    Client,
    api::{Api, DynamicObject, ListParams},
    config::Config,
    core::{ApiResource, GroupVersionKind},
    discovery,
};
use serde_json::Value;
use tokio::runtime::Runtime;

use crate::dynamic_object::DynamicObject as EngineObject;

pub fn list(resource: &str) -> Result<Vec<EngineObject>, String> {
    let resource = resource.trim();
    if resource.is_empty() {
        return Err("resource name is empty".to_string());
    }

    let runtime =
        Runtime::new().map_err(|error| format!("failed to init async runtime: {error}"))?;
    runtime.block_on(async_list(resource))
}

async fn async_list(resource: &str) -> Result<Vec<EngineObject>, String> {
    let mut config = Config::infer()
        .await
        .map_err(|error| format!("failed to infer kube config: {error}"))?;
    config.proxy_url = None;

    let client = Client::try_from(config)
        .map_err(|error| format!("failed to build kube client: {error}"))?;

    let api_resource = resolve_api_resource(&client, resource).await?;
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);

    let items = api
        .list(&ListParams::default())
        .await
        .map_err(|error| format!("failed to list resource '{resource}': {error}"))?;

    Ok(items
        .items
        .into_iter()
        .map(dynamic_to_engine_object)
        .collect())
}

async fn resolve_api_resource(
    client: &Client,
    resource: &str,
) -> Result<ApiResource, String> {
    let discovery = discovery::Discovery::new(client.clone())
        .run()
        .await
        .map_err(|error| format!("discovery failed: {error}"))?;

    for group in discovery.groups() {
        for (api_resource, capabilities) in group.recommended_resources() {
            if api_resource.plural.eq_ignore_ascii_case(resource) {
                let gvk = GroupVersionKind::gvk(
                    &api_resource.group,
                    &api_resource.version,
                    &api_resource.kind,
                );
                let resolved = ApiResource::from_gvk_with_plural(&gvk, &api_resource.plural);
                let _ = capabilities;
                return Ok(resolved);
            }
        }
    }

    Err(format!("resource '{resource}' was not found via discovery"))
}

fn dynamic_to_engine_object(object: DynamicObject) -> EngineObject {
    let mut fields = BTreeMap::new();
    let mut root = serde_json::Map::new();

    root.insert(
        "metadata".to_string(),
        serde_json::to_value(object.metadata).unwrap_or(Value::Null),
    );

    if let Value::Object(map) = object.data {
        for (key, value) in map {
            root.insert(key, value);
        }
    } else {
        root.insert("data".to_string(), object.data);
    }

    flatten_value("", &Value::Object(root), &mut fields);
    EngineObject { fields }
}

fn flatten_value(
    path: &str,
    value: &Value,
    out: &mut BTreeMap<String, String>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                flatten_value(&child_path, child, out);
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                let child_path = if path.is_empty() {
                    index.to_string()
                } else {
                    format!("{path}.{index}")
                };
                flatten_value(&child_path, child, out);
            }

            if !path.is_empty() {
                out.insert(path.to_string(), value.to_string());
            }
        }
        Value::String(string) => {
            if !path.is_empty() {
                out.insert(path.to_string(), string.clone());
            }
        }
        Value::Bool(boolean) => {
            if !path.is_empty() {
                out.insert(path.to_string(), boolean.to_string());
            }
        }
        Value::Number(number) => {
            if !path.is_empty() {
                out.insert(path.to_string(), number.to_string());
            }
        }
        Value::Null => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::flatten_value;

    #[test]
    fn flattens_nested_objects_to_dot_paths() {
        let mut out = std::collections::BTreeMap::new();
        let value = json!({
            "metadata": {
                "namespace": "demo-a"
            },
            "spec": {
                "replicas": 2,
                "enabled": true
            }
        });

        flatten_value("", &value, &mut out);

        assert_eq!(
            out.get("metadata.namespace").map(String::as_str),
            Some("demo-a")
        );
        assert_eq!(out.get("spec.replicas").map(String::as_str), Some("2"));
        assert_eq!(out.get("spec.enabled").map(String::as_str), Some("true"));
    }
}
