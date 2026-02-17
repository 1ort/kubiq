use crate::{dynamic_object::DynamicObject, parser};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct QueryPlan {
    pub predicates: Vec<parser::Predicate>,
}

pub fn build_plan(ast: parser::QueryAst) -> QueryPlan {
    QueryPlan {
        predicates: ast.predicates,
    }
}

pub fn evaluate(
    plan: &QueryPlan,
    objects: &[DynamicObject],
) -> Vec<DynamicObject> {
    objects
        .iter()
        .filter(|object| matches_all(object, &plan.predicates))
        .cloned()
        .collect()
}

fn matches_all(
    object: &DynamicObject,
    predicates: &[parser::Predicate],
) -> bool {
    predicates.iter().all(|predicate| {
        let value = object.get(&predicate.path).and_then(|value| comparable_eq(value, &predicate.value));

        match predicate.op {
            parser::Operator::Eq => value == Some(true),
            parser::Operator::Ne => value == Some(false),
        }
    })
}

fn comparable_eq(
    actual: &Value,
    expected: &Value,
) -> Option<bool> {
    match (actual, expected) {
        (Value::String(left), Value::String(right)) => Some(left == right),
        (Value::Number(left), Value::Number(right)) => Some(left == right),
        (Value::Bool(left), Value::Bool(right)) => Some(left == right),
        (Value::Null, _) | (_, Value::Null) => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use serde_json::Value;

    use crate::{
        dynamic_object::DynamicObject,
        parser::{Operator, Predicate},
    };

    use super::{QueryPlan, evaluate};

    #[test]
    fn keeps_only_matching_objects() {
        let mut fields_ok = BTreeMap::new();
        fields_ok.insert(
            "metadata.namespace".to_string(),
            Value::String("default".to_string()),
        );

        let mut fields_bad = BTreeMap::new();
        fields_bad.insert(
            "metadata.namespace".to_string(),
            Value::String("kube-system".to_string()),
        );

        let plan = QueryPlan {
            predicates: vec![Predicate {
                path: "metadata.namespace".to_string(),
                op: Operator::Eq,
                value: Value::String("default".to_string()),
            }],
        };

        let result = evaluate(
            &plan,
            &[
                DynamicObject { fields: fields_ok },
                DynamicObject { fields: fields_bad },
            ],
        );

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn missing_field_does_not_match_eq_or_ne() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "metadata.namespace".to_string(),
            Value::String("default".to_string()),
        );

        let object = DynamicObject { fields };

        let eq_plan = QueryPlan {
            predicates: vec![Predicate {
                path: "spec.nodeName".to_string(),
                op: Operator::Eq,
                value: Value::String("worker-1".to_string()),
            }],
        };

        let ne_plan = QueryPlan {
            predicates: vec![Predicate {
                path: "spec.nodeName".to_string(),
                op: Operator::Ne,
                value: Value::String("worker-1".to_string()),
            }],
        };

        assert!(evaluate(&eq_plan, std::slice::from_ref(&object)).is_empty());
        assert!(evaluate(&ne_plan, &[object]).is_empty());
    }

    #[test]
    fn type_mismatch_does_not_match_eq_or_ne() {
        let mut fields = BTreeMap::new();
        fields.insert("spec.replicas".to_string(), Value::from(2));
        let object = DynamicObject { fields };

        let eq_plan = QueryPlan {
            predicates: vec![Predicate {
                path: "spec.replicas".to_string(),
                op: Operator::Eq,
                value: Value::String("2".to_string()),
            }],
        };

        let ne_plan = QueryPlan {
            predicates: vec![Predicate {
                path: "spec.replicas".to_string(),
                op: Operator::Ne,
                value: Value::String("2".to_string()),
            }],
        };

        assert!(evaluate(&eq_plan, std::slice::from_ref(&object)).is_empty());
        assert!(evaluate(&ne_plan, &[object]).is_empty());
    }
}
