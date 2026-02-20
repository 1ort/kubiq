use std::cmp::Ordering;

use crate::dynamic_object::DynamicObject;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct QueryPlan {
    pub predicates: Vec<EnginePredicate>,
    pub select_paths: Option<Vec<String>>,
    pub sort_keys: Option<Vec<EngineSortKey>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnginePredicate {
    pub path: String,
    pub op: EngineOperator,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineOperator {
    Eq,
    Ne,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineSortKey {
    pub path: String,
    pub direction: EngineSortDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineSortDirection {
    Asc,
    Desc,
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

pub fn sort_objects(
    plan: &QueryPlan,
    objects: &[DynamicObject],
) -> Vec<DynamicObject> {
    let mut sorted = objects.to_vec();

    let Some(sort_keys) = plan.sort_keys.as_deref() else {
        return sorted;
    };

    sorted.sort_by(|left, right| compare_objects(left, right, sort_keys));
    sorted
}

fn compare_objects(
    left: &DynamicObject,
    right: &DynamicObject,
    sort_keys: &[EngineSortKey],
) -> Ordering {
    for key in sort_keys {
        let ordering = compare_values(left.get(&key.path), right.get(&key.path), key.direction);

        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    Ordering::Equal
}

fn compare_values(
    left: Option<&Value>,
    right: Option<&Value>,
    direction: EngineSortDirection,
) -> Ordering {
    match (to_sort_value(left), to_sort_value(right)) {
        (SortValue::Nullish, SortValue::Nullish) => Ordering::Equal,
        (SortValue::Nullish, _) => match direction {
            EngineSortDirection::Asc => Ordering::Less,
            EngineSortDirection::Desc => Ordering::Greater,
        },
        (_, SortValue::Nullish) => match direction {
            EngineSortDirection::Asc => Ordering::Greater,
            EngineSortDirection::Desc => Ordering::Less,
        },
        (SortValue::Concrete(left), SortValue::Concrete(right)) => {
            compare_non_null_values(left, right, direction)
        }
    }
}

fn compare_non_null_values(
    left: &Value,
    right: &Value,
    direction: EngineSortDirection,
) -> Ordering {
    let left_rank = value_rank(left);
    let right_rank = value_rank(right);

    let mut ordering = left_rank.cmp(&right_rank);
    if ordering == Ordering::Equal {
        ordering = compare_same_rank(left, right);
    }

    match direction {
        EngineSortDirection::Asc => ordering,
        EngineSortDirection::Desc => ordering.reverse(),
    }
}

fn compare_same_rank(
    left: &Value,
    right: &Value,
) -> Ordering {
    match (left, right) {
        (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
        (Value::Number(left), Value::Number(right)) => compare_numbers(left, right),
        (Value::String(left), Value::String(right)) => left.cmp(right),
        _ => Ordering::Equal,
    }
}

fn compare_numbers(
    left: &serde_json::Number,
    right: &serde_json::Number,
) -> Ordering {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return left.cmp(&right);
    }

    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return left.cmp(&right);
    }

    match (left.as_f64(), right.as_f64()) {
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
        _ => Ordering::Equal,
    }
}

fn value_rank(value: &Value) -> u8 {
    match value {
        Value::Bool(_) => 0,
        Value::Number(_) => 1,
        Value::String(_) => 2,
        _ => 3,
    }
}

enum SortValue<'a> {
    Nullish,
    Concrete(&'a Value),
}

fn to_sort_value(value: Option<&Value>) -> SortValue<'_> {
    match value {
        Some(Value::Null) | None => SortValue::Nullish,
        Some(value) => SortValue::Concrete(value),
    }
}

fn matches_all(
    object: &DynamicObject,
    predicates: &[EnginePredicate],
) -> bool {
    predicates.iter().all(|predicate| {
        let value = object
            .get(&predicate.path)
            .and_then(|value| comparable_eq(value, &predicate.value));

        match predicate.op {
            EngineOperator::Eq => value == Some(true),
            EngineOperator::Ne => value == Some(false),
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
    use serde_json::Value;
    use std::collections::BTreeMap;

    use crate::dynamic_object::DynamicObject;

    use super::{
        EngineOperator, EnginePredicate, EngineSortDirection, EngineSortKey, QueryPlan, evaluate,
        sort_objects,
    };

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
            predicates: vec![EnginePredicate {
                path: "metadata.namespace".to_string(),
                op: EngineOperator::Eq,
                value: Value::String("default".to_string()),
            }],
            select_paths: None,
            sort_keys: None,
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
            predicates: vec![EnginePredicate {
                path: "spec.nodeName".to_string(),
                op: EngineOperator::Eq,
                value: Value::String("worker-1".to_string()),
            }],
            select_paths: None,
            sort_keys: None,
        };

        let ne_plan = QueryPlan {
            predicates: vec![EnginePredicate {
                path: "spec.nodeName".to_string(),
                op: EngineOperator::Ne,
                value: Value::String("worker-1".to_string()),
            }],
            select_paths: None,
            sort_keys: None,
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
            predicates: vec![EnginePredicate {
                path: "spec.replicas".to_string(),
                op: EngineOperator::Eq,
                value: Value::String("2".to_string()),
            }],
            select_paths: None,
            sort_keys: None,
        };

        let ne_plan = QueryPlan {
            predicates: vec![EnginePredicate {
                path: "spec.replicas".to_string(),
                op: EngineOperator::Ne,
                value: Value::String("2".to_string()),
            }],
            select_paths: None,
            sort_keys: None,
        };

        assert!(evaluate(&eq_plan, std::slice::from_ref(&object)).is_empty());
        assert!(evaluate(&ne_plan, &[object]).is_empty());
    }

    #[test]
    fn sorts_by_single_key_asc() {
        let objects = vec![
            object(&[("metadata.name", Value::String("pod-c".to_string()))]),
            object(&[("metadata.name", Value::String("pod-a".to_string()))]),
            object(&[("metadata.name", Value::String("pod-b".to_string()))]),
        ];

        let plan = QueryPlan {
            predicates: Vec::new(),
            select_paths: None,
            sort_keys: Some(vec![EngineSortKey {
                path: "metadata.name".to_string(),
                direction: EngineSortDirection::Asc,
            }]),
        };

        let sorted = sort_objects(&plan, &objects);
        let names = names(&sorted);
        assert_eq!(names, vec!["pod-a", "pod-b", "pod-c"]);
    }

    #[test]
    fn sorts_by_single_key_desc() {
        let objects = vec![
            object(&[("spec.priority", Value::from(1))]),
            object(&[("spec.priority", Value::from(3))]),
            object(&[("spec.priority", Value::from(2))]),
        ];

        let plan = QueryPlan {
            predicates: Vec::new(),
            select_paths: None,
            sort_keys: Some(vec![EngineSortKey {
                path: "spec.priority".to_string(),
                direction: EngineSortDirection::Desc,
            }]),
        };

        let sorted = sort_objects(&plan, &objects);
        let priorities = values(&sorted, "spec.priority");
        assert_eq!(
            priorities,
            vec![Value::from(3), Value::from(2), Value::from(1)]
        );
    }

    #[test]
    fn sorts_nullish_sql_style() {
        let objects = vec![
            object(&[
                ("spec.rank", Value::from(2)),
                ("metadata.name", Value::String("c".to_string())),
            ]),
            object(&[("metadata.name", Value::String("a".to_string()))]),
            object(&[
                ("spec.rank", Value::Null),
                ("metadata.name", Value::String("b".to_string())),
            ]),
            object(&[
                ("spec.rank", Value::from(1)),
                ("metadata.name", Value::String("d".to_string())),
            ]),
        ];

        let asc_plan = QueryPlan {
            predicates: Vec::new(),
            select_paths: None,
            sort_keys: Some(vec![EngineSortKey {
                path: "spec.rank".to_string(),
                direction: EngineSortDirection::Asc,
            }]),
        };

        let desc_plan = QueryPlan {
            predicates: Vec::new(),
            select_paths: None,
            sort_keys: Some(vec![EngineSortKey {
                path: "spec.rank".to_string(),
                direction: EngineSortDirection::Desc,
            }]),
        };

        let asc = names(&sort_objects(&asc_plan, &objects));
        let desc = names(&sort_objects(&desc_plan, &objects));

        assert_eq!(asc, vec!["a", "b", "d", "c"]);
        assert_eq!(desc, vec!["c", "d", "a", "b"]);
    }

    #[test]
    fn sorts_mixed_types_with_fixed_precedence() {
        let objects = vec![
            object(&[
                ("spec.value", Value::String("z".to_string())),
                ("metadata.name", Value::String("s".to_string())),
            ]),
            object(&[
                ("spec.value", Value::from(10)),
                ("metadata.name", Value::String("n".to_string())),
            ]),
            object(&[
                ("spec.value", Value::Bool(true)),
                ("metadata.name", Value::String("b".to_string())),
            ]),
            object(&[
                ("spec.value", serde_json::json!({"k": "v"})),
                ("metadata.name", Value::String("o".to_string())),
            ]),
        ];

        let plan = QueryPlan {
            predicates: Vec::new(),
            select_paths: None,
            sort_keys: Some(vec![EngineSortKey {
                path: "spec.value".to_string(),
                direction: EngineSortDirection::Asc,
            }]),
        };

        let sorted = names(&sort_objects(&plan, &objects));
        assert_eq!(sorted, vec!["b", "n", "s", "o"]);
    }

    #[test]
    fn sorts_by_multiple_keys_and_is_stable() {
        let objects = vec![
            object(&[
                ("spec.rank", Value::from(1)),
                ("metadata.name", Value::String("beta".to_string())),
            ]),
            object(&[
                ("spec.rank", Value::from(1)),
                ("metadata.name", Value::String("alpha".to_string())),
            ]),
            object(&[
                ("spec.rank", Value::from(2)),
                ("metadata.name", Value::String("gamma".to_string())),
            ]),
            object(&[("spec.rank", Value::from(1))]),
        ];

        let plan = QueryPlan {
            predicates: Vec::new(),
            select_paths: None,
            sort_keys: Some(vec![
                EngineSortKey {
                    path: "spec.rank".to_string(),
                    direction: EngineSortDirection::Asc,
                },
                EngineSortKey {
                    path: "metadata.name".to_string(),
                    direction: EngineSortDirection::Asc,
                },
            ]),
        };

        let sorted = sort_objects(&plan, &objects);
        let names = names(&sorted);

        assert_eq!(names, vec!["-", "alpha", "beta", "gamma"]);
    }

    fn object(entries: &[(&str, Value)]) -> DynamicObject {
        let mut fields = BTreeMap::new();
        for (path, value) in entries {
            fields.insert((*path).to_string(), value.clone());
        }
        DynamicObject { fields }
    }

    fn names(objects: &[DynamicObject]) -> Vec<String> {
        objects
            .iter()
            .map(|object| {
                object
                    .fields
                    .get("metadata.name")
                    .and_then(Value::as_str)
                    .unwrap_or("-")
                    .to_string()
            })
            .collect()
    }

    fn values(
        objects: &[DynamicObject],
        path: &str,
    ) -> Vec<Value> {
        objects
            .iter()
            .map(|object| object.fields.get(path).cloned().unwrap_or(Value::Null))
            .collect()
    }
}
