use crate::{k8s::ListQueryOptions, parser};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushdownPlan {
    pub options: ListQueryOptions,
    pub diagnostics: Vec<PlannerDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerDiagnostic {
    pub path: String,
    pub op: parser::Operator,
    pub reason: NotPushableReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotPushableReason {
    UnsupportedPath,
    UnsupportedOperator,
    NonStringValue,
    UnsafeSelectorValue,
    UnsafeLabelKey,
}

pub fn plan_pushdown(predicates: &[parser::Predicate]) -> PushdownPlan {
    let mut field_selectors = Vec::new();
    let mut label_selectors = Vec::new();
    let mut diagnostics = Vec::new();

    for predicate in predicates {
        match predicate_to_selector(predicate) {
            Ok(SelectorTarget::Field(selector)) => field_selectors.push(selector),
            Ok(SelectorTarget::Label(selector)) => label_selectors.push(selector),
            Err(reason) => diagnostics.push(PlannerDiagnostic {
                path: predicate.path.clone(),
                op: predicate.op.clone(),
                reason,
            }),
        }
    }

    PushdownPlan {
        options: ListQueryOptions {
            field_selector: join_selector_parts(field_selectors),
            label_selector: join_selector_parts(label_selectors),
        },
        diagnostics,
    }
}

enum SelectorTarget {
    Field(String),
    Label(String),
}

fn predicate_to_selector(
    predicate: &parser::Predicate
) -> Result<SelectorTarget, NotPushableReason> {
    let operator = selector_operator(&predicate.op)?;
    let value = selector_value(&predicate.value).ok_or(NotPushableReason::NonStringValue)?;
    if !is_selector_value_safe(&value) {
        return Err(NotPushableReason::UnsafeSelectorValue);
    }

    if predicate.path.eq_ignore_ascii_case("metadata.name")
        || predicate.path.eq_ignore_ascii_case("metadata.namespace")
    {
        return Ok(SelectorTarget::Field(format!(
            "{}{operator}{value}",
            predicate.path
        )));
    }

    if let Some(label_key) = predicate.path.strip_prefix("metadata.labels.") {
        if !is_label_key_safe(label_key) {
            return Err(NotPushableReason::UnsafeLabelKey);
        }
        return Ok(SelectorTarget::Label(format!(
            "{label_key}{operator}{value}"
        )));
    }

    Err(NotPushableReason::UnsupportedPath)
}

fn selector_operator(op: &parser::Operator) -> Result<&'static str, NotPushableReason> {
    match op {
        parser::Operator::Eq => Ok("="),
        parser::Operator::Ne => Ok("!="),
    }
}

fn selector_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn is_selector_value_safe(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(',')
        && !value.contains('=')
        && !value.contains('!')
        && !value.chars().any(char::is_whitespace)
}

fn is_label_key_safe(key: &str) -> bool {
    !key.is_empty() && !key.contains(',') && !key.chars().any(char::is_whitespace)
}

fn join_selector_parts(parts: Vec<String>) -> Option<String> {
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::parser::{Operator, Predicate};

    use super::{NotPushableReason, plan_pushdown};

    #[test]
    fn pushes_field_selectors_for_eq_and_ne() {
        let predicates = vec![
            Predicate {
                path: "metadata.name".to_string(),
                op: Operator::Eq,
                value: Value::String("pod-a".to_string()),
            },
            Predicate {
                path: "metadata.namespace".to_string(),
                op: Operator::Ne,
                value: Value::String("kube-system".to_string()),
            },
        ];

        let plan = plan_pushdown(&predicates);
        assert_eq!(
            plan.options.field_selector.as_deref(),
            Some("metadata.name=pod-a,metadata.namespace!=kube-system")
        );
        assert_eq!(plan.options.label_selector, None);
        assert!(plan.diagnostics.is_empty());
    }

    #[test]
    fn pushes_label_selectors_for_eq_and_ne() {
        let predicates = vec![
            Predicate {
                path: "metadata.labels.app".to_string(),
                op: Operator::Eq,
                value: Value::String("api".to_string()),
            },
            Predicate {
                path: "metadata.labels.tier".to_string(),
                op: Operator::Ne,
                value: Value::String("batch".to_string()),
            },
        ];

        let plan = plan_pushdown(&predicates);
        assert_eq!(plan.options.field_selector, None);
        assert_eq!(
            plan.options.label_selector.as_deref(),
            Some("app=api,tier!=batch")
        );
        assert!(plan.diagnostics.is_empty());
    }

    #[test]
    fn reports_non_string_and_unsupported_path_as_not_pushable() {
        let predicates = vec![
            Predicate {
                path: "spec.replicas".to_string(),
                op: Operator::Eq,
                value: Value::from(3),
            },
            Predicate {
                path: "spec.nodeName".to_string(),
                op: Operator::Eq,
                value: Value::String("worker-a".to_string()),
            },
        ];

        let plan = plan_pushdown(&predicates);
        assert_eq!(plan.options.field_selector, None);
        assert_eq!(plan.options.label_selector, None);
        assert_eq!(plan.diagnostics.len(), 2);
        assert_eq!(
            plan.diagnostics[0].reason,
            NotPushableReason::NonStringValue
        );
        assert_eq!(
            plan.diagnostics[1].reason,
            NotPushableReason::UnsupportedPath
        );
    }

    #[test]
    fn reports_unsafe_selector_inputs() {
        let predicates = vec![
            Predicate {
                path: "metadata.name".to_string(),
                op: Operator::Eq,
                value: Value::String("pod,a".to_string()),
            },
            Predicate {
                path: "metadata.labels.bad,key".to_string(),
                op: Operator::Eq,
                value: Value::String("ok".to_string()),
            },
        ];

        let plan = plan_pushdown(&predicates);
        assert_eq!(plan.diagnostics.len(), 2);
        assert_eq!(
            plan.diagnostics[0].reason,
            NotPushableReason::UnsafeSelectorValue
        );
        assert_eq!(
            plan.diagnostics[1].reason,
            NotPushableReason::UnsafeLabelKey
        );
    }
}
