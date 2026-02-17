use crate::{dynamic_object::DynamicObject, parser};

#[derive(Clone, Debug, PartialEq, Eq)]
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
        let value = object.get(&predicate.path);

        match predicate.op {
            parser::Operator::Eq => value == Some(predicate.value.as_str()),
            parser::Operator::Ne => value.is_some_and(|actual| actual != predicate.value),
        }
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        dynamic_object::DynamicObject,
        parser::{Operator, Predicate},
    };

    use super::{QueryPlan, evaluate};

    #[test]
    fn keeps_only_matching_objects() {
        let mut fields_ok = BTreeMap::new();
        fields_ok.insert("metadata.namespace".to_string(), "default".to_string());

        let mut fields_bad = BTreeMap::new();
        fields_bad.insert("metadata.namespace".to_string(), "kube-system".to_string());

        let plan = QueryPlan {
            predicates: vec![Predicate {
                path: "metadata.namespace".to_string(),
                op: Operator::Eq,
                value: "default".to_string(),
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
        fields.insert("metadata.namespace".to_string(), "default".to_string());

        let object = DynamicObject { fields };

        let eq_plan = QueryPlan {
            predicates: vec![Predicate {
                path: "spec.nodeName".to_string(),
                op: Operator::Eq,
                value: "worker-1".to_string(),
            }],
        };

        let ne_plan = QueryPlan {
            predicates: vec![Predicate {
                path: "spec.nodeName".to_string(),
                op: Operator::Ne,
                value: "worker-1".to_string(),
            }],
        };

        assert!(evaluate(&eq_plan, std::slice::from_ref(&object)).is_empty());
        assert!(evaluate(&ne_plan, &[object]).is_empty());
    }
}
