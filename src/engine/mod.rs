use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::dynamic_object::DynamicObject;
use crate::error::EngineError;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct QueryPlan {
    pub predicates: Vec<EnginePredicate>,
    pub selection: Option<EngineSelection>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineSelection {
    Paths(Vec<String>),
    Aggregations(Vec<EngineAggregationExpr>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineAggregationExpr {
    pub function: EngineAggregationFunction,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineAggregationFunction {
    Count,
    Sum,
    Min,
    Max,
    Avg,
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

pub fn aggregate(
    plan: &QueryPlan,
    objects: &[DynamicObject],
) -> Result<Vec<DynamicObject>, EngineError> {
    let Some(EngineSelection::Aggregations(expressions)) = &plan.selection else {
        return Ok(objects.to_vec());
    };

    let mut row = BTreeMap::new();
    for expression in expressions {
        let key = aggregation_key(expression);
        let value = evaluate_aggregation(expression, objects)?;
        row.insert(key, value);
    }

    Ok(vec![DynamicObject { fields: row }])
}

fn evaluate_aggregation(
    expression: &EngineAggregationExpr,
    objects: &[DynamicObject],
) -> Result<Value, EngineError> {
    match expression.function {
        EngineAggregationFunction::Count => count_aggregation(expression.path.as_deref(), objects),
        EngineAggregationFunction::Sum => sum_aggregation(required_path(expression)?, objects),
        EngineAggregationFunction::Min => min_max_aggregation(required_path(expression)?, objects, true),
        EngineAggregationFunction::Max => min_max_aggregation(required_path(expression)?, objects, false),
        EngineAggregationFunction::Avg => avg_aggregation(required_path(expression)?, objects),
    }
}

fn required_path(expression: &EngineAggregationExpr) -> Result<&str, EngineError> {
    expression
        .path
        .as_deref()
        .ok_or_else(|| EngineError::InvalidAggregation {
            function: aggregation_function_name(&expression.function).to_string(),
            path: "*".to_string(),
            expected: "path argument",
            actual: "none".to_string(),
        })
}

fn aggregation_key(expression: &EngineAggregationExpr) -> String {
    let function = aggregation_function_name(&expression.function);
    match expression.path.as_deref() {
        Some(path) => format!("{function}({path})"),
        None => format!("{function}(*)"),
    }
}

fn aggregation_function_name(function: &EngineAggregationFunction) -> &'static str {
    match function {
        EngineAggregationFunction::Count => "count",
        EngineAggregationFunction::Sum => "sum",
        EngineAggregationFunction::Min => "min",
        EngineAggregationFunction::Max => "max",
        EngineAggregationFunction::Avg => "avg",
    }
}

fn count_aggregation(
    path: Option<&str>,
    objects: &[DynamicObject],
) -> Result<Value, EngineError> {
    let count = if let Some(path) = path {
        objects
            .iter()
            .filter(|object| object.get(path).is_some_and(|value| !value.is_null()))
            .count()
    } else {
        objects.len()
    };

    Ok(Value::from(count as u64))
}

fn sum_aggregation(
    path: &str,
    objects: &[DynamicObject],
) -> Result<Value, EngineError> {
    let mut total_i128: i128 = 0;
    let mut total_f64: f64 = 0.0;
    let mut use_float = false;
    let mut has_value = false;

    for object in objects {
        let Some(value) = object.get(path) else {
            continue;
        };
        if value.is_null() {
            continue;
        }

        let Some(number) = numeric_from_json(value) else {
            return Err(non_numeric_aggregation_error("sum", path, value));
        };

        has_value = true;
        match number {
            NumericValue::Int(value) if !use_float => {
                total_i128 = total_i128.checked_add(value).ok_or_else(|| {
                    EngineError::InvalidAggregation {
                        function: "sum".to_string(),
                        path: path.to_string(),
                        expected: "representable integer sum",
                        actual: "overflow".to_string(),
                    }
                })?;
            }
            NumericValue::Int(value) => total_f64 += value as f64,
            NumericValue::Float(value) => {
                if !use_float {
                    total_f64 = total_i128 as f64;
                    use_float = true;
                }
                total_f64 += value;
            }
        }
    }

    if !has_value {
        return Ok(Value::from(0));
    }

    if !use_float {
        return integer_to_json_number("sum", path, total_i128);
    }

    serde_json::Number::from_f64(total_f64)
        .map(Value::Number)
        .ok_or_else(|| EngineError::InvalidAggregation {
            function: "sum".to_string(),
            path: path.to_string(),
            expected: "finite numeric result",
            actual: "non-finite".to_string(),
        })
}

fn avg_aggregation(
    path: &str,
    objects: &[DynamicObject],
) -> Result<Value, EngineError> {
    let mut sum_i128: i128 = 0;
    let mut sum_f64: f64 = 0.0;
    let mut use_float = false;
    let mut count = 0usize;

    for object in objects {
        let Some(value) = object.get(path) else {
            continue;
        };
        if value.is_null() {
            continue;
        }

        let Some(number) = numeric_from_json(value) else {
            return Err(non_numeric_aggregation_error("avg", path, value));
        };
        match number {
            NumericValue::Int(value) if !use_float => {
                sum_i128 = sum_i128.checked_add(value).ok_or_else(|| {
                    EngineError::InvalidAggregation {
                        function: "avg".to_string(),
                        path: path.to_string(),
                        expected: "representable integer sum",
                        actual: "overflow".to_string(),
                    }
                })?;
            }
            NumericValue::Int(value) => sum_f64 += value as f64,
            NumericValue::Float(value) => {
                if !use_float {
                    sum_f64 = sum_i128 as f64;
                    use_float = true;
                }
                sum_f64 += value;
            }
        }
        count += 1;
    }

    if count == 0 {
        return Ok(Value::Null);
    }

    if !use_float {
        sum_f64 = sum_i128 as f64;
    }

    let average = sum_f64 / count as f64;
    serde_json::Number::from_f64(average)
        .map(Value::Number)
        .ok_or_else(|| EngineError::InvalidAggregation {
            function: "avg".to_string(),
            path: path.to_string(),
            expected: "finite numeric result",
            actual: "non-finite".to_string(),
        })
}

fn min_max_aggregation(
    path: &str,
    objects: &[DynamicObject],
    is_min: bool,
) -> Result<Value, EngineError> {
    let mut best: Option<&Value> = None;
    let mut best_type: Option<&'static str> = None;

    for object in objects {
        let Some(value) = object.get(path) else {
            continue;
        };
        if value.is_null() {
            continue;
        }

        let value_type = comparable_type(value).ok_or_else(|| EngineError::InvalidAggregation {
            function: if is_min { "min" } else { "max" }.to_string(),
            path: path.to_string(),
            expected: "bool, number, or string",
            actual: value_type_name(value).to_string(),
        })?;

        if let Some(current_type) = best_type
            && current_type != value_type
        {
            return Err(EngineError::IncompatibleAggregationTypes {
                function: if is_min { "min" } else { "max" }.to_string(),
                path: path.to_string(),
                left: current_type.to_string(),
                right: value_type.to_string(),
            });
        }

        if let Some(current) = best {
            let ordering = compare_same_type_values(current, value)?;
            if (is_min && ordering == Ordering::Greater) || (!is_min && ordering == Ordering::Less)
            {
                best = Some(value);
            }
        } else {
            best = Some(value);
            best_type = Some(value_type);
        }
    }

    Ok(best.cloned().unwrap_or(Value::Null))
}

fn compare_same_type_values(
    left: &Value,
    right: &Value,
) -> Result<Ordering, EngineError> {
    match (left, right) {
        (Value::Bool(left), Value::Bool(right)) => Ok(left.cmp(right)),
        (Value::String(left), Value::String(right)) => Ok(left.cmp(right)),
        (Value::Number(left), Value::Number(right)) => compare_number_values(left, right),
        _ => Err(EngineError::InvalidAggregation {
            function: "min/max".to_string(),
            path: "<internal>".to_string(),
            expected: "comparable primitive values",
            actual: "mixed or unsupported types".to_string(),
        }),
    }
}

fn comparable_type(value: &Value) -> Option<&'static str> {
    match value {
        Value::Bool(_) => Some("bool"),
        Value::Number(_) => Some("number"),
        Value::String(_) => Some("string"),
        _ => None,
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

enum NumericValue {
    Int(i128),
    Float(f64),
}

fn numeric_from_json(value: &Value) -> Option<NumericValue> {
    let number = value.as_number()?;
    if let Some(value) = number.as_i64() {
        return Some(NumericValue::Int(i128::from(value)));
    }
    if let Some(value) = number.as_u64() {
        return Some(NumericValue::Int(i128::from(value)));
    }
    number.as_f64().map(NumericValue::Float)
}

fn integer_to_json_number(
    function: &str,
    path: &str,
    value: i128,
) -> Result<Value, EngineError> {
    if value >= i128::from(i64::MIN) && value <= i128::from(i64::MAX) {
        return Ok(Value::from(value as i64));
    }
    if value >= 0 && value <= i128::from(u64::MAX) {
        return Ok(Value::from(value as u64));
    }
    Err(EngineError::InvalidAggregation {
        function: function.to_string(),
        path: path.to_string(),
        expected: "representable JSON integer",
        actual: "out of range".to_string(),
    })
}

fn compare_number_values(
    left: &serde_json::Number,
    right: &serde_json::Number,
) -> Result<Ordering, EngineError> {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return Ok(left.cmp(&right));
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return Ok(left.cmp(&right));
    }
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_u64()) {
        return Ok(if left < 0 {
            Ordering::Less
        } else {
            (left as u64).cmp(&right)
        });
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_i64()) {
        return Ok(if right < 0 {
            Ordering::Greater
        } else {
            left.cmp(&(right as u64))
        });
    }

    let Some(left) = left.as_f64() else {
        return Err(EngineError::InvalidAggregation {
            function: "min/max".to_string(),
            path: "<internal>".to_string(),
            expected: "finite numeric value",
            actual: "non-finite".to_string(),
        });
    };
    let Some(right) = right.as_f64() else {
        return Err(EngineError::InvalidAggregation {
            function: "min/max".to_string(),
            path: "<internal>".to_string(),
            expected: "finite numeric value",
            actual: "non-finite".to_string(),
        });
    };

    Ok(left.partial_cmp(&right).unwrap_or(Ordering::Equal))
}

fn non_numeric_aggregation_error(
    function: &str,
    path: &str,
    value: &Value,
) -> EngineError {
    EngineError::InvalidAggregation {
        function: function.to_string(),
        path: path.to_string(),
        expected: "number",
        actual: value_type_name(value).to_string(),
    }
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
        EngineAggregationExpr, EngineAggregationFunction, EngineOperator, EnginePredicate,
        EngineSelection, EngineSortDirection, EngineSortKey, QueryPlan, aggregate, evaluate,
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
            selection: None,
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
            selection: None,
            sort_keys: None,
        };

        let ne_plan = QueryPlan {
            predicates: vec![EnginePredicate {
                path: "spec.nodeName".to_string(),
                op: EngineOperator::Ne,
                value: Value::String("worker-1".to_string()),
            }],
            selection: None,
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
            selection: None,
            sort_keys: None,
        };

        let ne_plan = QueryPlan {
            predicates: vec![EnginePredicate {
                path: "spec.replicas".to_string(),
                op: EngineOperator::Ne,
                value: Value::String("2".to_string()),
            }],
            selection: None,
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
            selection: None,
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
            selection: None,
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
            selection: None,
            sort_keys: Some(vec![EngineSortKey {
                path: "spec.rank".to_string(),
                direction: EngineSortDirection::Asc,
            }]),
        };

        let desc_plan = QueryPlan {
            predicates: Vec::new(),
            selection: None,
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
            selection: None,
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
            selection: None,
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

    #[test]
    fn aggregates_count_sum_min_max_avg() {
        let objects = vec![
            object(&[("spec.replicas", Value::from(1))]),
            object(&[("spec.replicas", Value::from(3))]),
            object(&[("spec.replicas", Value::from(2))]),
        ];
        let plan = QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(vec![
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Count,
                    path: None,
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Sum,
                    path: Some("spec.replicas".to_string()),
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Min,
                    path: Some("spec.replicas".to_string()),
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Max,
                    path: Some("spec.replicas".to_string()),
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Avg,
                    path: Some("spec.replicas".to_string()),
                },
            ])),
            sort_keys: None,
        };

        let rows = aggregate(&plan, &objects).expect("must aggregate");
        assert_eq!(rows.len(), 1);
        let row = &rows[0].fields;
        assert_eq!(row.get("count(*)"), Some(&Value::from(3)));
        assert_eq!(row.get("sum(spec.replicas)"), Some(&Value::from(6)));
        assert_eq!(row.get("min(spec.replicas)"), Some(&Value::from(1)));
        assert_eq!(row.get("max(spec.replicas)"), Some(&Value::from(3)));
        assert_eq!(row.get("avg(spec.replicas)"), Some(&Value::from(2.0)));
    }

    #[test]
    fn aggregates_empty_set_sql_like() {
        let plan = QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(vec![
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Count,
                    path: None,
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Count,
                    path: Some("spec.replicas".to_string()),
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Sum,
                    path: Some("spec.replicas".to_string()),
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Avg,
                    path: Some("spec.replicas".to_string()),
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Min,
                    path: Some("spec.replicas".to_string()),
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Max,
                    path: Some("spec.replicas".to_string()),
                },
            ])),
            sort_keys: None,
        };

        let rows = aggregate(&plan, &[]).expect("must aggregate");
        let row = &rows[0].fields;
        assert_eq!(row.get("count(*)"), Some(&Value::from(0)));
        assert_eq!(row.get("count(spec.replicas)"), Some(&Value::from(0)));
        assert_eq!(row.get("sum(spec.replicas)"), Some(&Value::from(0)));
        assert_eq!(row.get("avg(spec.replicas)"), Some(&Value::Null));
        assert_eq!(row.get("min(spec.replicas)"), Some(&Value::Null));
        assert_eq!(row.get("max(spec.replicas)"), Some(&Value::Null));
    }

    #[test]
    fn aggregate_sum_errors_on_non_numeric_values() {
        let objects = vec![object(&[("spec.replicas", Value::String("bad".to_string()))])];
        let plan = QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(vec![EngineAggregationExpr {
                function: EngineAggregationFunction::Sum,
                path: Some("spec.replicas".to_string()),
            }])),
            sort_keys: None,
        };

        let err = aggregate(&plan, &objects).expect_err("must fail");
        assert!(err.to_string().contains("expects number"));
    }

    #[test]
    fn aggregate_min_errors_on_mixed_types() {
        let objects = vec![
            object(&[("spec.value", Value::from(10))]),
            object(&[("spec.value", Value::String("x".to_string()))]),
        ];
        let plan = QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(vec![EngineAggregationExpr {
                function: EngineAggregationFunction::Min,
                path: Some("spec.value".to_string()),
            }])),
            sort_keys: None,
        };

        let err = aggregate(&plan, &objects).expect_err("must fail");
        assert!(err.to_string().contains("cannot compare mixed types"));
    }

    #[test]
    fn aggregate_count_path_ignores_missing_and_null() {
        let objects = vec![
            object(&[("spec.replicas", Value::from(3))]),
            object(&[("spec.replicas", Value::Null)]),
            object(&[]),
        ];
        let plan = QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(vec![EngineAggregationExpr {
                function: EngineAggregationFunction::Count,
                path: Some("spec.replicas".to_string()),
            }])),
            sort_keys: None,
        };

        let rows = aggregate(&plan, &objects).expect("must aggregate");
        let row = &rows[0].fields;
        assert_eq!(row.get("count(spec.replicas)"), Some(&Value::from(1)));
    }

    #[test]
    fn aggregate_sum_keeps_large_integer_precision() {
        let objects = vec![
            object(&[("spec.value", Value::from(9_007_199_254_740_993u64))]),
            object(&[("spec.value", Value::from(2u64))]),
        ];
        let plan = QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(vec![EngineAggregationExpr {
                function: EngineAggregationFunction::Sum,
                path: Some("spec.value".to_string()),
            }])),
            sort_keys: None,
        };

        let rows = aggregate(&plan, &objects).expect("must aggregate");
        let row = &rows[0].fields;
        assert_eq!(
            row.get("sum(spec.value)"),
            Some(&Value::from(9_007_199_254_740_995u64))
        );
    }

    #[test]
    fn aggregate_min_max_compare_large_integers_exactly() {
        let objects = vec![
            object(&[("spec.value", Value::from(9_007_199_254_740_993u64))]),
            object(&[("spec.value", Value::from(9_007_199_254_740_992u64))]),
        ];
        let plan = QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(vec![
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Min,
                    path: Some("spec.value".to_string()),
                },
                EngineAggregationExpr {
                    function: EngineAggregationFunction::Max,
                    path: Some("spec.value".to_string()),
                },
            ])),
            sort_keys: None,
        };

        let rows = aggregate(&plan, &objects).expect("must aggregate");
        let row = &rows[0].fields;
        assert_eq!(
            row.get("min(spec.value)"),
            Some(&Value::from(9_007_199_254_740_992u64))
        );
        assert_eq!(
            row.get("max(spec.value)"),
            Some(&Value::from(9_007_199_254_740_993u64))
        );
    }

    #[test]
    fn aggregate_avg_supports_float_values() {
        let objects = vec![
            object(&[("spec.value", Value::from(1.5))]),
            object(&[("spec.value", Value::from(2.5))]),
        ];
        let plan = QueryPlan {
            predicates: Vec::new(),
            selection: Some(EngineSelection::Aggregations(vec![EngineAggregationExpr {
                function: EngineAggregationFunction::Avg,
                path: Some("spec.value".to_string()),
            }])),
            sort_keys: None,
        };

        let rows = aggregate(&plan, &objects).expect("must aggregate");
        let row = &rows[0].fields;
        assert_eq!(row.get("avg(spec.value)"), Some(&Value::from(2.0)));
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
