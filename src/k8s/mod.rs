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

const LIST_PAGE_SIZE: u32 = 500;
const MAX_LIST_PAGES: usize = 10_000;

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
    let config = Config::infer()
        .await
        .map_err(|error| format!("failed to infer kube config: {error}"))?;

    let client = Client::try_from(config)
        .map_err(|error| format!("failed to build kube client: {error}"))?;

    let api_resource = resolve_api_resource(&client, resource).await?;
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);

    let mut all_items = Vec::new();
    let mut continue_token: Option<String> = None;
    let mut page_count: usize = 0;

    loop {
        page_count += 1;
        if page_count > MAX_LIST_PAGES {
            return Err(format!(
                "pagination for resource '{resource}' exceeded max pages ({MAX_LIST_PAGES})"
            ));
        }

        let params = build_list_params(LIST_PAGE_SIZE, continue_token.as_deref());
        let mut page = api
            .list(&params)
            .await
            .map_err(|error| format!("failed to list resource '{resource}': {error}"))?;

        all_items.append(&mut page.items);

        let next_continue = page.metadata.continue_.filter(|token| !token.is_empty());
        if next_continue.is_none() {
            break;
        }

        if continue_token.as_ref() == next_continue.as_ref() {
            return Err(format!(
                "pagination for resource '{resource}' got stuck on continue token '{token}'",
                token = next_continue.as_deref().unwrap_or_default()
            ));
        }

        continue_token = next_continue;
    }

    Ok(all_items
        .into_iter()
        .map(dynamic_to_engine_object)
        .collect())
}

fn build_list_params(
    limit: u32,
    continue_token: Option<&str>,
) -> ListParams {
    let mut params = ListParams::default().limit(limit);
    if let Some(token) = continue_token {
        params = params.continue_token(token);
    }
    params
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
    out: &mut BTreeMap<String, Value>,
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
                out.insert(path.to_string(), value.clone());
            }
        }
        Value::String(string) => {
            if !path.is_empty() {
                out.insert(path.to_string(), Value::String(string.clone()));
            }
        }
        Value::Bool(boolean) => {
            if !path.is_empty() {
                out.insert(path.to_string(), Value::Bool(*boolean));
            }
        }
        Value::Number(number) => {
            if !path.is_empty() {
                out.insert(path.to_string(), Value::Number(number.clone()));
            }
        }
        Value::Null => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{MAX_LIST_PAGES, build_list_params, flatten_value};

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
            out.get("metadata.namespace"),
            Some(&Value::String("demo-a".to_string()))
        );
        assert_eq!(out.get("spec.replicas"), Some(&Value::from(2)));
        assert_eq!(out.get("spec.enabled"), Some(&Value::Bool(true)));
    }

    #[test]
    fn builds_list_params_with_limit_and_continue_token() {
        let params = build_list_params(250, Some("next-token"));
        assert_eq!(params.limit, Some(250));
        assert_eq!(params.continue_token.as_deref(), Some("next-token"));
    }

    #[test]
    fn builds_list_params_with_limit_only() {
        let params = build_list_params(250, None);
        assert_eq!(params.limit, Some(250));
        assert_eq!(params.continue_token, None);
    }

    #[test]
    fn max_pages_guard_is_sane_for_large_clusters() {
        assert!(MAX_LIST_PAGES >= 10_000);
    }
}
