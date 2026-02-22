pub mod planner;

use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    sync::{OnceLock, RwLock},
    time::{Duration, Instant},
};

use kube::{
    Client,
    api::{Api, DynamicObject, ListParams},
    config::Config,
    core::{ApiResource, GroupVersionKind},
    discovery,
};
use serde_json::Value;
use tokio::{
    runtime::Runtime,
    time::{sleep, timeout},
};

use crate::{
    dynamic_object::DynamicObject as EngineObject,
    error::{K8sError, RetryErrorKind, RetryStopReason, boxed_error},
};

const LIST_PAGE_SIZE: u32 = 500;
const MAX_LIST_PAGES: usize = 10_000;
const DISCOVERY_CACHE_TTL: Duration = Duration::from_secs(60);
const RETRY_MAX_ATTEMPTS: usize = 3;
const RETRY_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
const RETRY_MAX_BACKOFF: Duration = Duration::from_millis(400);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DiscoveryCacheKey {
    cluster_identity: String,
    namespace: String,
    resource: String,
}

impl DiscoveryCacheKey {
    fn new(
        cluster_identity: String,
        namespace: String,
        resource: &str,
    ) -> Self {
        Self {
            cluster_identity,
            namespace,
            resource: normalize_resource(resource),
        }
    }

    fn from_config(
        config: &Config,
        resource: &str,
    ) -> Self {
        let namespace = if config.default_namespace.is_empty() {
            "default".to_string()
        } else {
            config.default_namespace.clone()
        };
        Self::new(config.cluster_url.to_string(), namespace, resource)
    }
}

#[derive(Clone, Debug)]
struct DiscoveryCacheEntry {
    api_resource: ApiResource,
    expires_at: Instant,
}

static DISCOVERY_CACHE: OnceLock<RwLock<HashMap<DiscoveryCacheKey, DiscoveryCacheEntry>>> =
    OnceLock::new();

fn discovery_cache() -> &'static RwLock<HashMap<DiscoveryCacheKey, DiscoveryCacheEntry>> {
    DISCOVERY_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[derive(Clone, Copy, Debug)]
struct RetryPolicy {
    max_attempts: usize,
    initial_backoff: Duration,
    max_backoff: Duration,
    request_timeout: Duration,
}

const DEFAULT_RETRY_POLICY: RetryPolicy = RetryPolicy {
    max_attempts: RETRY_MAX_ATTEMPTS,
    initial_backoff: RETRY_INITIAL_BACKOFF,
    max_backoff: RETRY_MAX_BACKOFF,
    request_timeout: REQUEST_TIMEOUT,
};

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ListResult {
    pub objects: Vec<EngineObject>,
    pub diagnostics: Vec<K8sDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum K8sDiagnostic {
    SelectorFallback {
        reason: SelectorFallbackReason,
        attempted: ListQueryOptions,
    },
    RetrySummary {
        stage: &'static str,
        attempts: usize,
        reason: RetryStopReason,
        final_error: RetryErrorKind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorFallbackReason {
    ApiRejectedBadRequest,
}

pub fn retry_summary_diagnostic(error: &K8sError) -> Option<K8sDiagnostic> {
    match error {
        K8sError::RetryExhausted {
            stage,
            attempts,
            reason,
            final_error,
            ..
        } => Some(K8sDiagnostic::RetrySummary {
            stage,
            attempts: *attempts,
            reason: *reason,
            final_error: *final_error,
        }),
        _ => None,
    }
}

pub fn list(
    resource: &str,
    options: &ListQueryOptions,
) -> Result<ListResult, K8sError> {
    let runtime = Runtime::new().map_err(|source| K8sError::RuntimeInit { source })?;
    runtime.block_on(list_async(resource, options))
}

pub async fn list_async(
    resource: &str,
    options: &ListQueryOptions,
) -> Result<ListResult, K8sError> {
    let resource = normalize_resource(resource);
    if resource.is_empty() {
        return Err(K8sError::EmptyResourceName);
    }

    let config = Config::infer()
        .await
        .map_err(|source| K8sError::ConfigInfer {
            source: boxed_error(source),
        })?;

    let cache_key = DiscoveryCacheKey::from_config(&config, &resource);
    let client = Client::try_from(config).map_err(|source| K8sError::ClientBuild {
        source: boxed_error(source),
    })?;

    let mut api_resource = resolve_api_resource_cached(&client, &cache_key).await?;
    let mut api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);

    let (items, diagnostics) = match list_with_selector_fallback(&resource, &api, options).await {
        Ok(result) => result,
        Err(error) if should_retry_with_fresh_discovery(&error) => {
            invalidate_discovery_cache(&cache_key);
            api_resource = resolve_api_resource_cached(&client, &cache_key).await?;
            api = Api::all_with(client.clone(), &api_resource);
            list_with_selector_fallback(&resource, &api, options).await?
        }
        Err(error) => return Err(error),
    };

    Ok(ListResult {
        objects: items.into_iter().map(dynamic_to_engine_object).collect(),
        diagnostics,
    })
}

async fn list_with_selector_fallback(
    resource: &str,
    api: &Api<DynamicObject>,
    options: &ListQueryOptions,
) -> Result<(Vec<DynamicObject>, Vec<K8sDiagnostic>), K8sError> {
    let mut diagnostics = Vec::new();
    let items = match list_pages(resource, api, options).await {
        Ok(items) => items,
        Err(error) if options.has_selectors() && should_retry_without_selectors(&error) => {
            diagnostics.push(K8sDiagnostic::SelectorFallback {
                reason: SelectorFallbackReason::ApiRejectedBadRequest,
                attempted: options.clone(),
            });
            list_pages(resource, api, &ListQueryOptions::default()).await?
        }
        Err(error) => return Err(error),
    };

    Ok((items, diagnostics))
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
        let mut page = run_with_retry(
            "list",
            &DEFAULT_RETRY_POLICY,
            || api.list(&params),
            |source| map_list_error(resource, options.has_selectors(), source),
            is_retryable_kube_error,
        )
        .await?;

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

fn retry_backoff_for_attempt(
    policy: &RetryPolicy,
    attempt: usize,
) -> Duration {
    let shift = attempt.saturating_sub(1).min(8);
    let base_millis = policy.initial_backoff.as_millis() as u64;
    let cap_millis = policy.max_backoff.as_millis() as u64;
    let next_millis = base_millis.saturating_mul(1_u64 << shift).min(cap_millis);
    Duration::from_millis(next_millis)
}

fn retry_error_kind(error: &K8sError) -> RetryErrorKind {
    match error {
        K8sError::ApiUnreachable { .. } => RetryErrorKind::ApiUnreachable,
        K8sError::RequestTimeout { .. } => RetryErrorKind::RequestTimeout,
        K8sError::SelectorRejected { .. } => RetryErrorKind::SelectorRejected,
        K8sError::ResourceResolutionStale { .. } => RetryErrorKind::ResourceResolutionStale,
        K8sError::ListFailed { .. } => RetryErrorKind::ListFailed,
        K8sError::DiscoveryRun { .. } => RetryErrorKind::DiscoveryRun,
        _ => RetryErrorKind::Other,
    }
}

fn is_retryable_kube_error(source: &kube::Error) -> bool {
    match source {
        kube::Error::Service(_) | kube::Error::HyperError(_) => true,
        kube::Error::Api(error) => error.code == 429 || error.code >= 500,
        _ => false,
    }
}

async fn run_with_retry<T, Op, Fut, Map, Classify>(
    stage: &'static str,
    policy: &RetryPolicy,
    mut operation: Op,
    mut map_error: Map,
    mut classify: Classify,
) -> Result<T, K8sError>
where
    Op: FnMut() -> Fut,
    Fut: Future<Output = Result<T, kube::Error>>,
    Map: FnMut(kube::Error) -> K8sError,
    Classify: FnMut(&kube::Error) -> bool,
{
    let mut attempt: usize = 1;

    loop {
        let result = timeout(policy.request_timeout, operation()).await;
        match result {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(source)) => {
                let retryable = classify(&source);
                let mapped = map_error(source);

                if retryable && attempt < policy.max_attempts {
                    sleep(retry_backoff_for_attempt(policy, attempt)).await;
                    attempt += 1;
                    continue;
                }

                if retryable {
                    return Err(K8sError::RetryExhausted {
                        stage,
                        attempts: attempt,
                        reason: RetryStopReason::RetryCapReached,
                        final_error: retry_error_kind(&mapped),
                        source: boxed_error(mapped),
                    });
                }

                if attempt > 1 {
                    return Err(K8sError::RetryExhausted {
                        stage,
                        attempts: attempt,
                        reason: RetryStopReason::NonRetryable,
                        final_error: retry_error_kind(&mapped),
                        source: boxed_error(mapped),
                    });
                }

                return Err(mapped);
            }
            Err(source) => {
                let timed_out = K8sError::RequestTimeout {
                    stage,
                    timeout_ms: policy.request_timeout.as_millis() as u64,
                    source,
                };

                if attempt < policy.max_attempts {
                    sleep(retry_backoff_for_attempt(policy, attempt)).await;
                    attempt += 1;
                    continue;
                }

                return Err(K8sError::RetryExhausted {
                    stage,
                    attempts: attempt,
                    reason: RetryStopReason::RetryCapReached,
                    final_error: RetryErrorKind::RequestTimeout,
                    source: boxed_error(timed_out),
                });
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListErrorClass {
    SelectorRejected,
    ResourceResolutionStale,
    ApiUnreachable,
    Other,
}

fn classify_list_error(
    source: &kube::Error,
    had_selectors: bool,
) -> ListErrorClass {
    match source {
        kube::Error::Api(error) if had_selectors && is_selector_rejection(error) => {
            ListErrorClass::SelectorRejected
        }
        kube::Error::Api(error) if is_resource_resolution_stale(error) => {
            ListErrorClass::ResourceResolutionStale
        }
        kube::Error::Service(_) | kube::Error::HyperError(_) => ListErrorClass::ApiUnreachable,
        _ => ListErrorClass::Other,
    }
}

fn is_selector_rejection(error: &kube::error::ErrorResponse) -> bool {
    if error.code != 400 {
        return false;
    }

    let reason = error.reason.to_ascii_lowercase();
    let message = error.message.to_ascii_lowercase();
    const SELECTOR_MARKERS: [&str; 4] = [
        "field selector",
        "label selector",
        "not a known field selector",
        "field label not supported",
    ];

    SELECTOR_MARKERS
        .iter()
        .any(|marker| reason.contains(marker) || message.contains(marker))
}

fn is_resource_resolution_stale(error: &kube::error::ErrorResponse) -> bool {
    matches!(error.code, 404 | 410)
}

fn map_list_error(
    resource: &str,
    had_selectors: bool,
    source: kube::Error,
) -> K8sError {
    match classify_list_error(&source, had_selectors) {
        ListErrorClass::SelectorRejected => K8sError::SelectorRejected {
            resource: resource.to_string(),
            source: boxed_error(source),
        },
        ListErrorClass::ResourceResolutionStale => K8sError::ResourceResolutionStale {
            resource: resource.to_string(),
            source: boxed_error(source),
        },
        ListErrorClass::ApiUnreachable => K8sError::ApiUnreachable {
            stage: "list",
            source: boxed_error(source),
        },
        ListErrorClass::Other => K8sError::ListFailed {
            resource: resource.to_string(),
            source: boxed_error(source),
        },
    }
}

fn should_retry_without_selectors(error: &K8sError) -> bool {
    matches!(error, K8sError::SelectorRejected { .. })
}

fn should_retry_with_fresh_discovery(error: &K8sError) -> bool {
    matches!(
        error,
        K8sError::ResourceResolutionStale { .. }
            | K8sError::RetryExhausted {
                final_error: RetryErrorKind::ResourceResolutionStale,
                ..
            }
    )
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
    let discovery = run_with_retry(
        "discovery",
        &DEFAULT_RETRY_POLICY,
        || discovery::Discovery::new(client.clone()).run(),
        map_discovery_error,
        is_retryable_kube_error,
    )
    .await?;

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

async fn resolve_api_resource_cached(
    client: &Client,
    key: &DiscoveryCacheKey,
) -> Result<ApiResource, K8sError> {
    if let Some(api_resource) = cache_lookup(key) {
        return Ok(api_resource);
    }

    let api_resource = resolve_api_resource(client, &key.resource).await?;
    cache_insert(key.clone(), api_resource.clone(), DISCOVERY_CACHE_TTL);
    Ok(api_resource)
}

fn cache_lookup(key: &DiscoveryCacheKey) -> Option<ApiResource> {
    let now = Instant::now();
    {
        let cache = discovery_cache()
            .read()
            .expect("discovery cache read lock must not be poisoned");
        if let Some(entry) = cache.get(key) {
            if now <= entry.expires_at {
                return Some(entry.api_resource.clone());
            }
        } else {
            return None;
        }
    }

    let mut cache = discovery_cache()
        .write()
        .expect("discovery cache write lock must not be poisoned");
    if cache
        .get(key)
        .is_some_and(|entry| Instant::now() > entry.expires_at)
    {
        cache.remove(key);
    }
    None
}

fn cache_insert(
    key: DiscoveryCacheKey,
    api_resource: ApiResource,
    ttl: Duration,
) {
    let entry = DiscoveryCacheEntry {
        api_resource,
        expires_at: Instant::now() + ttl,
    };
    discovery_cache()
        .write()
        .expect("discovery cache write lock must not be poisoned")
        .insert(key, entry);
}

fn invalidate_discovery_cache(key: &DiscoveryCacheKey) {
    discovery_cache()
        .write()
        .expect("discovery cache write lock must not be poisoned")
        .remove(key);
}

fn normalize_resource(resource: &str) -> String {
    resource.trim().to_ascii_lowercase()
}

fn map_discovery_error(source: kube::Error) -> K8sError {
    match classify_list_error(&source, false) {
        ListErrorClass::ApiUnreachable => K8sError::ApiUnreachable {
            stage: "discovery",
            source: boxed_error(source),
        },
        _ => K8sError::DiscoveryRun {
            source: boxed_error(source),
        },
    }
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
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };

    use kube::core::GroupVersionKind;
    use serde_json::{Value, json};

    use super::{
        DiscoveryCacheEntry, DiscoveryCacheKey, K8sDiagnostic, ListErrorClass, ListQueryOptions,
        RetryPolicy, DEFAULT_RETRY_POLICY, MAX_LIST_PAGES, SelectorFallbackReason,
        build_list_params, cache_insert, cache_lookup, classify_list_error, discovery_cache,
        ensure_page_limit, flatten_value, invalidate_discovery_cache, is_retryable_kube_error,
        list_async, next_continue_token, normalize_resource, retry_backoff_for_attempt,
        run_with_retry, should_retry_with_fresh_discovery, should_retry_without_selectors,
    };
    use crate::error::{K8sError, RetryErrorKind, RetryStopReason};

    fn clear_discovery_cache() {
        discovery_cache()
            .write()
            .expect("discovery cache write lock must not be poisoned")
            .clear();
    }

    fn dummy_api_resource() -> kube::core::ApiResource {
        let gvk = GroupVersionKind::gvk("apps", "v1", "Deployment");
        kube::core::ApiResource::from_gvk_with_plural(&gvk, "deployments")
    }

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
    fn empty_resource_name_is_typed_error_async() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime init must succeed");
        let result = runtime.block_on(list_async("  ", &ListQueryOptions::default()));
        assert!(matches!(result, Err(K8sError::EmptyResourceName)));
    }

    #[test]
    fn classifies_api_bad_request_as_selector_rejected() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "field selector not supported".to_string(),
            reason: "BadRequest".to_string(),
            code: 400,
        });
        assert_eq!(
            classify_list_error(&error, true),
            ListErrorClass::SelectorRejected
        );
    }

    #[test]
    fn classifies_non_selector_bad_request_as_other() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "invalid request body".to_string(),
            reason: "BadRequest".to_string(),
            code: 400,
        });
        assert_eq!(classify_list_error(&error, true), ListErrorClass::Other);
    }

    #[test]
    fn classifies_selector_bad_request_as_other_without_selectors() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "field selector not supported".to_string(),
            reason: "BadRequest".to_string(),
            code: 400,
        });
        assert_eq!(classify_list_error(&error, false), ListErrorClass::Other);
    }

    #[test]
    fn classifies_service_error_as_api_unreachable() {
        let error = kube::Error::Service(std::io::Error::other("connect").into());
        assert_eq!(
            classify_list_error(&error, false),
            ListErrorClass::ApiUnreachable
        );
    }

    #[test]
    fn classifies_other_api_errors_as_other() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "forbidden".to_string(),
            reason: "Forbidden".to_string(),
            code: 403,
        });
        assert_eq!(classify_list_error(&error, false), ListErrorClass::Other);
    }

    #[test]
    fn classifies_not_found_as_resource_resolution_stale() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "the server could not find the requested resource".to_string(),
            reason: "NotFound".to_string(),
            code: 404,
        });
        assert_eq!(
            classify_list_error(&error, false),
            ListErrorClass::ResourceResolutionStale
        );
    }

    #[test]
    fn classifies_gone_as_resource_resolution_stale() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "resource no longer available".to_string(),
            reason: "Gone".to_string(),
            code: 410,
        });
        assert_eq!(
            classify_list_error(&error, false),
            ListErrorClass::ResourceResolutionStale
        );
    }

    #[test]
    fn normalizes_resource_to_trimmed_lowercase() {
        assert_eq!(normalize_resource("  Pods "), "pods");
    }

    #[test]
    fn discovery_cache_key_normalizes_resource_segment() {
        let key = DiscoveryCacheKey::new("cluster-a".to_string(), "default".to_string(), " PoDs ");
        assert_eq!(key.resource, "pods");
    }

    #[test]
    fn cache_lookup_returns_inserted_entry_before_expiry() {
        clear_discovery_cache();
        let key = DiscoveryCacheKey::new("cluster-a".to_string(), "default".to_string(), "pods");
        let api_resource = dummy_api_resource();
        cache_insert(key.clone(), api_resource.clone(), Duration::from_secs(30));

        let cached = cache_lookup(&key).expect("cache hit expected");
        assert_eq!(cached.plural, api_resource.plural);
    }

    #[test]
    fn cache_lookup_drops_expired_entry() {
        clear_discovery_cache();
        let key = DiscoveryCacheKey::new("cluster-a".to_string(), "default".to_string(), "pods");
        let api_resource = dummy_api_resource();
        discovery_cache()
            .write()
            .expect("discovery cache write lock must not be poisoned")
            .insert(
                key.clone(),
                DiscoveryCacheEntry {
                    api_resource,
                    expires_at: Instant::now() - Duration::from_secs(1),
                },
            );

        assert!(cache_lookup(&key).is_none());
        let cache = discovery_cache()
            .read()
            .expect("discovery cache read lock must not be poisoned");
        assert!(!cache.contains_key(&key));
    }

    #[test]
    fn invalidate_discovery_cache_removes_entry() {
        clear_discovery_cache();
        let key = DiscoveryCacheKey::new("cluster-a".to_string(), "default".to_string(), "pods");
        cache_insert(key.clone(), dummy_api_resource(), Duration::from_secs(30));
        invalidate_discovery_cache(&key);

        assert!(cache_lookup(&key).is_none());
    }

    #[test]
    fn retries_without_selectors_on_selector_rejected_errors() {
        let error = K8sError::SelectorRejected {
            resource: "pods".to_string(),
            source: crate::error::boxed_error(std::io::Error::other("bad request")),
        };
        assert!(should_retry_without_selectors(&error));
    }

    #[test]
    fn retries_with_fresh_discovery_on_stale_resolution_errors() {
        let error = K8sError::ResourceResolutionStale {
            resource: "pods".to_string(),
            source: crate::error::boxed_error(std::io::Error::other("stale mapping")),
        };
        assert!(should_retry_with_fresh_discovery(&error));
    }

    #[test]
    fn retries_with_fresh_discovery_on_retry_exhausted_stale_resolution() {
        let error = K8sError::RetryExhausted {
            stage: "list",
            attempts: 3,
            reason: RetryStopReason::RetryCapReached,
            final_error: RetryErrorKind::ResourceResolutionStale,
            source: crate::error::boxed_error(std::io::Error::other("stale mapping")),
        };
        assert!(should_retry_with_fresh_discovery(&error));
    }

    #[test]
    fn classifies_api_429_as_retryable() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "too many requests".to_string(),
            reason: "TooManyRequests".to_string(),
            code: 429,
        });
        assert!(is_retryable_kube_error(&error));
    }

    #[test]
    fn classifies_api_500_as_retryable() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "internal".to_string(),
            reason: "InternalError".to_string(),
            code: 500,
        });
        assert!(is_retryable_kube_error(&error));
    }

    #[test]
    fn classifies_api_400_as_non_retryable() {
        let error = kube::Error::Api(kube::error::ErrorResponse {
            status: "Failure".to_string(),
            message: "bad request".to_string(),
            reason: "BadRequest".to_string(),
            code: 400,
        });
        assert!(!is_retryable_kube_error(&error));
    }

    #[test]
    fn computes_exponential_backoff_with_cap() {
        assert_eq!(
            retry_backoff_for_attempt(&DEFAULT_RETRY_POLICY, 1),
            Duration::from_millis(100)
        );
        assert_eq!(
            retry_backoff_for_attempt(&DEFAULT_RETRY_POLICY, 2),
            Duration::from_millis(200)
        );
        assert_eq!(
            retry_backoff_for_attempt(&DEFAULT_RETRY_POLICY, 3),
            Duration::from_millis(400)
        );
        assert_eq!(
            retry_backoff_for_attempt(&DEFAULT_RETRY_POLICY, 4),
            Duration::from_millis(400)
        );
    }

    #[test]
    fn run_with_retry_succeeds_after_transient_error() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime init must succeed");
        let attempts = Arc::new(AtomicUsize::new(0));
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(2),
            request_timeout: Duration::from_millis(20),
        };

        let result = runtime.block_on(run_with_retry(
            "list",
            &policy,
            {
                let attempts = Arc::clone(&attempts);
                move || {
                    let attempts = Arc::clone(&attempts);
                    async move {
                        let current = attempts.fetch_add(1, Ordering::SeqCst);
                        if current == 0 {
                            Err(kube::Error::Service(std::io::Error::other("connect").into()))
                        } else {
                            Ok(7_u8)
                        }
                    }
                }
            },
            |source| super::map_list_error("pods", false, source),
            super::is_retryable_kube_error,
        ));

        assert_eq!(result.expect("must succeed after retry"), 7_u8);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn run_with_retry_fails_fast_for_non_retryable_error() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime init must succeed");
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(2),
            request_timeout: Duration::from_millis(20),
        };

        let result: Result<u8, K8sError> = runtime.block_on(run_with_retry(
            "list",
            &policy,
            || async {
                Err(kube::Error::Api(kube::error::ErrorResponse {
                    status: "Failure".to_string(),
                    message: "forbidden".to_string(),
                    reason: "Forbidden".to_string(),
                    code: 403,
                }))
            },
            |source| super::map_list_error("pods", false, source),
            super::is_retryable_kube_error,
        ));

        assert!(matches!(result, Err(K8sError::ListFailed { .. })));
    }

    #[test]
    fn run_with_retry_returns_retry_exhausted_on_retry_cap() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime init must succeed");
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(2),
            request_timeout: Duration::from_millis(20),
        };

        let result: Result<u8, K8sError> = runtime.block_on(run_with_retry(
            "list",
            &policy,
            || async {
                Err(kube::Error::Service(
                    std::io::Error::other("dial tcp timeout").into(),
                ))
            },
            |source| super::map_list_error("pods", false, source),
            super::is_retryable_kube_error,
        ));

        assert!(matches!(
            result,
            Err(K8sError::RetryExhausted {
                stage: "list",
                attempts: 3,
                reason: RetryStopReason::RetryCapReached,
                final_error: RetryErrorKind::ApiUnreachable,
                ..
            })
        ));
    }

    #[test]
    fn run_with_retry_reports_timeout_path() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime init must succeed");
        let policy = RetryPolicy {
            max_attempts: 2,
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(2),
            request_timeout: Duration::from_millis(5),
        };

        let result = runtime.block_on(run_with_retry(
            "list",
            &policy,
            || async {
                tokio::time::sleep(Duration::from_millis(25)).await;
                Ok(1_u8)
            },
            |source| super::map_list_error("pods", false, source),
            super::is_retryable_kube_error,
        ));

        assert!(matches!(
            result,
            Err(K8sError::RetryExhausted {
                stage: "list",
                attempts: 2,
                reason: RetryStopReason::RetryCapReached,
                final_error: RetryErrorKind::RequestTimeout,
                ..
            })
        ));
    }

    #[test]
    fn builds_retry_summary_diagnostic_from_retry_exhausted_error() {
        let error = K8sError::RetryExhausted {
            stage: "list",
            attempts: 3,
            reason: RetryStopReason::RetryCapReached,
            final_error: RetryErrorKind::RequestTimeout,
            source: crate::error::boxed_error(std::io::Error::other("timeout")),
        };

        let diagnostic = super::retry_summary_diagnostic(&error).expect("must produce diagnostic");
        assert!(matches!(
            diagnostic,
            K8sDiagnostic::RetrySummary {
                stage: "list",
                attempts: 3,
                reason: RetryStopReason::RetryCapReached,
                final_error: RetryErrorKind::RequestTimeout,
            }
        ));
    }

    #[test]
    fn selector_fallback_diagnostic_keeps_attempted_selectors() {
        let diagnostic = K8sDiagnostic::SelectorFallback {
            reason: SelectorFallbackReason::ApiRejectedBadRequest,
            attempted: ListQueryOptions {
                field_selector: Some("metadata.namespace=demo-a".to_string()),
                label_selector: None,
            },
        };

        assert!(matches!(
            diagnostic,
            K8sDiagnostic::SelectorFallback {
                reason: SelectorFallbackReason::ApiRejectedBadRequest,
                attempted: ListQueryOptions {
                    field_selector: Some(_),
                    label_selector: None,
                }
            }
        ));
    }
}
