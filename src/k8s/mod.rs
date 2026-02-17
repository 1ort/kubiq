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

use crate::{
    dynamic_object::DynamicObject as EngineObject,
    error::{K8sError, boxed_error},
};

const LIST_PAGE_SIZE: u32 = 500;
const MAX_LIST_PAGES: usize = 10_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListQueryOptions {
    pub field_selector: Option<String>,
    pub label_selector: Option<String>,
}

impl ListQueryOptions {
    fn has_selectors(&self) -> bool {
        self.field_selector.is_some() || self.label_selector.is_some()
    }
}

pub fn list(
    resource: &str,
    options: &ListQueryOptions,
) -> Result<Vec<EngineObject>, K8sError> {
    let resource = resource.trim();
    if resource.is_empty() {
        return Err(K8sError::EmptyResourceName);
    }

    let runtime = Runtime::new().map_err(|source| K8sError::RuntimeInit { source })?;
    runtime.block_on(async_list(resource, options))
}

async fn async_list(
    resource: &str,
    options: &ListQueryOptions,
) -> Result<Vec<EngineObject>, K8sError> {
    let config = Config::infer()
        .await
        .map_err(|source| K8sError::ConfigInfer {
            source: boxed_error(source),
        })?;

    let client = Client::try_from(config).map_err(|source| K8sError::ClientBuild {
        source: boxed_error(source),
    })?;

    let api_resource = resolve_api_resource(&client, resource).await?;
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);

    let items = match list_pages(resource, &api, options).await {
        Ok(items) => items,
        Err(error) if options.has_selectors() && should_retry_without_selectors(&error) => {
            list_pages(resource, &api, &ListQueryOptions::default()).await?
        }
        Err(error) => return Err(error),
    };

    Ok(items.into_iter().map(dynamic_to_engine_object).collect())
}

async fn list_pages(
    resource: &str,
    api: &Api<DynamicObject>,
    options: &ListQueryOptions,
) -> Result<Vec<DynamicObject>, K8sError> {
    let mut all_items = Vec::new();
    let mut continue_token: Option<String> = None;
    let mut page_count: usize = 0;

    loop {
        page_count += 1;
        ensure_page_limit(resource, page_count)?;

        let params = build_list_params(LIST_PAGE_SIZE, continue_token.as_deref(), options);
        let mut page = api
            .list(&params)
            .await
            .map_err(|source| map_list_error(resource, source))?;

        all_items.append(&mut page.items);
        continue_token =
            next_continue_token(resource, continue_token.as_deref(), page.metadata.continue_)?;
        if continue_token.is_none() {
            break;
        }
    }

    Ok(all_items)
}

fn build_list_params(
    limit: u32,
    continue_token: Option<&str>,
    options: &ListQueryOptions,
) -> ListParams {
    let mut params = ListParams::default().limit(limit);
    if let Some(token) = continue_token {
        params = params.continue_token(token);
    }
    if let Some(selector) = options.field_selector.as_deref() {
        params = params.fields(selector);
    }
    if let Some(selector) = options.label_selector.as_deref() {
        params = params.labels(selector);
    }
    params
}

fn map_list_error(
    resource: &str,
    source: kube::Error,
) -> K8sError {
    if is_api_unreachable_error_message(&source.to_string()) {
        return K8sError::ApiUnreachable {
            stage: "list",
            source: boxed_error(source),
        };
    }

    K8sError::ListFailed {
        resource: resource.to_string(),
        source: boxed_error(source),
    }
}

fn should_retry_without_selectors(error: &K8sError) -> bool {
    match error {
        K8sError::ListFailed { source, .. } => {
            let message = source.to_string();
            message.contains("BadRequest")
                || message.contains("field label not supported")
                || message.contains("not a known field selector")
                || message.contains("field selector")
        }
        _ => false,
    }
}

fn ensure_page_limit(
    resource: &str,
    page_count: usize,
) -> Result<(), K8sError> {
    if page_count > MAX_LIST_PAGES {
        return Err(K8sError::PaginationExceeded {
            resource: resource.to_string(),
            max_pages: MAX_LIST_PAGES,
        });
    }
    Ok(())
}

fn next_continue_token(
    resource: &str,
    current_token: Option<&str>,
    raw_next_token: Option<String>,
) -> Result<Option<String>, K8sError> {
    let next_token = raw_next_token.filter(|token| !token.is_empty());
    if next_token.is_none() {
        return Ok(None);
    }

    if current_token == next_token.as_deref() {
        return Err(K8sError::PaginationStuck {
            resource: resource.to_string(),
            token: next_token.as_deref().unwrap_or_default().to_string(),
        });
    }

    Ok(next_token)
}

async fn resolve_api_resource(
    client: &Client,
    resource: &str,
) -> Result<ApiResource, K8sError> {
    let discovery = discovery::Discovery::new(client.clone())
        .run()
        .await
        .map_err(map_discovery_error)?;

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

    Err(K8sError::ResourceNotFound {
        resource: resource.to_string(),
    })
}

fn map_discovery_error(source: kube::Error) -> K8sError {
    if is_api_unreachable_error_message(&source.to_string()) {
        return K8sError::ApiUnreachable {
            stage: "discovery",
            source: boxed_error(source),
        };
    }

    K8sError::DiscoveryRun {
        source: boxed_error(source),
    }
}

fn is_api_unreachable_error_message(message: &str) -> bool {
    message.contains("client error (Connect)") || message.contains("Unable to connect")
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

    use super::{
        ListQueryOptions, MAX_LIST_PAGES, build_list_params, ensure_page_limit, flatten_value,
        is_api_unreachable_error_message, next_continue_token, should_retry_without_selectors,
    };
    use crate::error::K8sError;

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
        let params = build_list_params(250, Some("next-token"), &ListQueryOptions::default());
        assert_eq!(params.limit, Some(250));
        assert_eq!(params.continue_token.as_deref(), Some("next-token"));
    }

    #[test]
    fn builds_list_params_with_limit_only() {
        let params = build_list_params(250, None, &ListQueryOptions::default());
        assert_eq!(params.limit, Some(250));
        assert_eq!(params.continue_token, None);
    }

    #[test]
    fn builds_list_params_with_selectors() {
        let params = build_list_params(
            250,
            None,
            &ListQueryOptions {
                field_selector: Some("metadata.namespace=demo-a".to_string()),
                label_selector: Some("app=api".to_string()),
            },
        );
        assert_eq!(
            params.field_selector.as_deref(),
            Some("metadata.namespace=demo-a")
        );
        assert_eq!(params.label_selector.as_deref(), Some("app=api"));
    }

    #[test]
    fn page_limit_accepts_boundary_value() {
        let result = ensure_page_limit("pods", MAX_LIST_PAGES);
        assert!(result.is_ok());
    }

    #[test]
    fn page_limit_rejects_overflow() {
        let result = ensure_page_limit("pods", MAX_LIST_PAGES + 1);
        assert!(matches!(
            result,
            Err(K8sError::PaginationExceeded {
                resource,
                max_pages
            }) if resource == "pods" && max_pages == MAX_LIST_PAGES
        ));
    }

    #[test]
    fn next_continue_token_treats_absent_as_done() {
        let result = next_continue_token("pods", None, None).expect("must succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn next_continue_token_treats_empty_as_done() {
        let result = next_continue_token("pods", Some("token-a"), Some(String::new()))
            .expect("must succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn next_continue_token_accepts_new_token() {
        let result = next_continue_token("pods", Some("token-a"), Some("token-b".to_string()))
            .expect("must succeed");
        assert_eq!(result.as_deref(), Some("token-b"));
    }

    #[test]
    fn next_continue_token_rejects_same_token() {
        let result = next_continue_token("pods", Some("token-a"), Some("token-a".to_string()));
        assert!(matches!(
            result,
            Err(K8sError::PaginationStuck { resource, token })
                if resource == "pods" && token == "token-a"
        ));
    }

    #[test]
    fn empty_resource_name_is_typed_error() {
        let result = super::list("  ", &ListQueryOptions::default());
        assert!(matches!(result, Err(K8sError::EmptyResourceName)));
    }

    #[test]
    fn recognizes_connect_error_message() {
        assert!(is_api_unreachable_error_message(
            "ServiceError: client error (Connect)"
        ));
    }

    #[test]
    fn recognizes_unable_to_connect_message() {
        assert!(is_api_unreachable_error_message(
            "Unable to connect to the server"
        ));
    }

    #[test]
    fn does_not_mark_other_messages_as_connectivity() {
        assert!(!is_api_unreachable_error_message(
            "forbidden: user is not authorized"
        ));
    }

    #[test]
    fn retries_without_selectors_on_bad_request_errors() {
        let error = K8sError::ListFailed {
            resource: "pods".to_string(),
            source: crate::error::boxed_error(std::io::Error::other(
                "BadRequest: field label not supported",
            )),
        };
        assert!(should_retry_without_selectors(&error));
    }
}
