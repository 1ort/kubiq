use std::error::Error as StdError;

use thiserror::Error;

type BoxError = Box<dyn StdError + Send + Sync>;

pub fn boxed_error<E>(error: E) -> BoxError
where
    E: StdError + Send + Sync + 'static,
{
    Box::new(error)
}

#[derive(Debug, Error)]
pub enum K8sError {
    #[error("resource name is empty")]
    EmptyResourceName,
    #[error("failed to init async runtime")]
    RuntimeInit {
        #[source]
        source: std::io::Error,
    },
    #[error("failed to infer kube config")]
    ConfigInfer {
        #[source]
        source: BoxError,
    },
    #[error("failed to build kube client")]
    ClientBuild {
        #[source]
        source: BoxError,
    },
    #[error("discovery failed")]
    DiscoveryRun {
        #[source]
        source: BoxError,
    },
    #[error("kubernetes api is unreachable during {stage}")]
    ApiUnreachable {
        stage: &'static str,
        #[source]
        source: BoxError,
    },
    #[error("resource '{resource}' was not found via discovery")]
    ResourceNotFound { resource: String },
    #[error("failed to list resource '{resource}'")]
    ListFailed {
        resource: String,
        #[source]
        source: BoxError,
    },
    #[error("pagination for resource '{resource}' exceeded max pages ({max_pages})")]
    PaginationExceeded { resource: String, max_pages: usize },
    #[error("pagination for resource '{resource}' got stuck on continue token '{token}'")]
    PaginationStuck { resource: String, token: String },
}

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("failed to serialize json output")]
    JsonSerialize {
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize yaml output")]
    YamlSerialize {
        #[source]
        source: serde_yaml::Error,
    },
}

#[derive(Debug)]
pub enum CliError {
    InvalidArgs(String),
    Parse(String),
    K8s(K8sError),
    Output(OutputError),
}

impl std::fmt::Display for CliError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::InvalidArgs(error) => write!(
                f,
                "invalid args: {error}\n\nTip: run `kubiq --help` to see usage and examples."
            ),
            Self::Parse(error) => write!(
                f,
                "parse error: {error}\n\nTip: query format is `<resource> where <predicates> [select <paths>]`.\nExample: `kubiq pods where metadata.namespace == demo-a select metadata.name`"
            ),
            Self::K8s(error) => write!(f, "k8s error: {error}\n\n{}", k8s_tip(error)),
            Self::Output(error) => write!(
                f,
                "output error: {error}\n\nTip: supported formats are `table`, `json`, `yaml`."
            ),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::K8s(error) => Some(error),
            Self::Output(error) => Some(error),
            _ => None,
        }
    }
}

fn k8s_tip(error: &K8sError) -> &'static str {
    match error {
        K8sError::ResourceNotFound { .. } => {
            "Tip: resource was not found. Check plural name via:\n  kubectl api-resources"
        }
        K8sError::ApiUnreachable { .. } => {
            "Tip: Kubernetes API is unreachable. Check context/cluster:\n  kubectl config current-context\n  kubectl cluster-info"
        }
        _ => "Tip: verify cluster access with `kubectl get ns` and then retry.",
    }
}
