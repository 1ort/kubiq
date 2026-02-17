#[derive(Debug, Clone, PartialEq, Eq)]
pub enum K8sError {
    EmptyResourceName,
    RuntimeInit(String),
    ConfigInfer(String),
    ClientBuild(String),
    DiscoveryRun(String),
    ResourceNotFound { resource: String },
    ListFailed { resource: String, source: String },
    PaginationExceeded { resource: String, max_pages: usize },
    PaginationStuck { resource: String, token: String },
}

impl std::fmt::Display for K8sError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::EmptyResourceName => write!(f, "resource name is empty"),
            Self::RuntimeInit(source) => write!(f, "failed to init async runtime: {source}"),
            Self::ConfigInfer(source) => write!(f, "failed to infer kube config: {source}"),
            Self::ClientBuild(source) => write!(f, "failed to build kube client: {source}"),
            Self::DiscoveryRun(source) => write!(f, "discovery failed: {source}"),
            Self::ResourceNotFound { resource } => {
                write!(f, "resource '{resource}' was not found via discovery")
            }
            Self::ListFailed { resource, source } => {
                write!(f, "failed to list resource '{resource}': {source}")
            }
            Self::PaginationExceeded {
                resource,
                max_pages,
            } => write!(
                f,
                "pagination for resource '{resource}' exceeded max pages ({max_pages})"
            ),
            Self::PaginationStuck { resource, token } => write!(
                f,
                "pagination for resource '{resource}' got stuck on continue token '{token}'"
            ),
        }
    }
}

impl std::error::Error for K8sError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputError {
    JsonSerialize(String),
    YamlSerialize(String),
}

impl std::fmt::Display for OutputError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::JsonSerialize(source) => {
                write!(f, "failed to serialize json output: {source}")
            }
            Self::YamlSerialize(source) => {
                write!(f, "failed to serialize yaml output: {source}")
            }
        }
    }
}

impl std::error::Error for OutputError {}

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl std::error::Error for CliError {}

fn k8s_tip(error: &K8sError) -> &'static str {
    match error {
        K8sError::ResourceNotFound { .. } => {
            "Tip: resource was not found. Check plural name via:\n  kubectl api-resources"
        }
        K8sError::DiscoveryRun(source) | K8sError::ListFailed { source, .. }
            if source.contains("client error (Connect)")
                || source.contains("Unable to connect") =>
        {
            "Tip: Kubernetes API is unreachable. Check context/cluster:\n  kubectl config current-context\n  kubectl cluster-info"
        }
        _ => "Tip: verify cluster access with `kubectl get ns` and then retry.",
    }
}
