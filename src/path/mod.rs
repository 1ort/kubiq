use std::collections::BTreeMap;

use serde_json::{Map, Value};

pub fn encode_segment(segment: &str) -> String {
    segment.replace('%', "%25").replace('.', "%2E")
}

pub fn decode_segment(segment: &str) -> String {
    let mut decoded = String::with_capacity(segment.len());
    let bytes = segment.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let code = &segment[index + 1..index + 3];
            if code.eq_ignore_ascii_case("2e") {
                decoded.push('.');
                index += 3;
                continue;
            }
            if code.eq_ignore_ascii_case("25") {
                decoded.push('%');
                index += 3;
                continue;
            }
        }

        decoded.push(bytes[index] as char);
        index += 1;
    }

    decoded
}

pub fn encode_path(path: &str) -> String {
    path.split('.')
        .map(encode_segment)
        .collect::<Vec<_>>()
        .join(".")
}

pub fn decode_path(path: &str) -> String {
    decode_parts(path).join(".")
}

pub fn flatten_json_to_fields(root: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    flatten_segments(&mut Vec::new(), root, &mut out);
    out
}

pub fn reconstruct_nested_from_fields(fields: &BTreeMap<String, Value>) -> Value {
    let mut root = Value::Object(Map::new());
    for (encoded_path, value) in fields {
        let parts = decode_parts(encoded_path);
        insert_nested_value(&mut root, &parts, value.clone());
    }
    root
}

pub fn select_path_value(
    fields: &BTreeMap<String, Value>,
    path: &str,
) -> Option<Value> {
    let encoded_path = encode_path(path);
    if let Some(value) = fields.get(&encoded_path) {
        return Some(value.clone());
    }

    let prefix = format!("{encoded_path}.");
    let mut nested = Value::Object(Map::new());
    let mut found = false;

    for (encoded_key, value) in fields {
        if let Some(encoded_suffix) = encoded_key.strip_prefix(&prefix) {
            if encoded_suffix.is_empty() {
                continue;
            }
            found = true;
            let parts = decode_parts(encoded_suffix);
            insert_nested_value(&mut nested, &parts, value.clone());
        }
    }

    if found { Some(nested) } else { None }
}

fn flatten_segments(
    path: &mut Vec<String>,
    value: &Value,
    out: &mut BTreeMap<String, Value>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                path.push(encode_segment(key));
                flatten_segments(path, child, out);
                path.pop();
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                path.push(index.to_string());
                flatten_segments(path, child, out);
                path.pop();
            }
            if !path.is_empty() {
                out.insert(path.join("."), value.clone());
            }
        }
        Value::String(_) | Value::Bool(_) | Value::Number(_) => {
            if !path.is_empty() {
                out.insert(path.join("."), value.clone());
            }
        }
        Value::Null => {}
    }
}

fn decode_parts(path: &str) -> Vec<String> {
    if path.is_empty() {
        Vec::new()
    } else {
        path.split('.').map(decode_segment).collect()
    }
}

fn insert_nested_value(
    node: &mut Value,
    parts: &[String],
    value: Value,
) {
    if parts.is_empty() {
        *node = value;
        return;
    }

    if let Ok(index) = parts[0].parse::<usize>() {
        if !node.is_array() {
            *node = Value::Array(Vec::new());
        }
        if let Value::Array(array) = node {
            while array.len() <= index {
                array.push(Value::Null);
            }
            insert_nested_value(&mut array[index], &parts[1..], value);
        }
        return;
    }

    if !node.is_object() {
        *node = Value::Object(Map::new());
    }
    if let Value::Object(map) = node {
        let entry = map.entry(parts[0].clone()).or_insert(Value::Null);
        insert_nested_value(entry, &parts[1..], value);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use super::{
        decode_path, decode_segment, encode_path, encode_segment, flatten_json_to_fields,
        reconstruct_nested_from_fields, select_path_value,
    };

    #[test]
    fn encodes_and_decodes_segments() {
        assert_eq!(encode_segment("annotations"), "annotations");
        assert_eq!(
            encode_segment("kubectl.kubernetes.io/restartedAt"),
            "kubectl%2Ekubernetes%2Eio/restartedAt"
        );
        assert_eq!(encode_segment("a%b"), "a%25b");
        assert_eq!(
            decode_segment("kubectl%2Ekubernetes%2Eio/restartedAt"),
            "kubectl.kubernetes.io/restartedAt"
        );
        assert_eq!(decode_segment("a%25b"), "a%b");
    }

    #[test]
    fn encodes_path_segment_wise() {
        assert_eq!(
            encode_path("metadata.annotations.kubectl.kubernetes.io/restartedAt"),
            "metadata.annotations.kubectl.kubernetes.io/restartedAt"
        );
        assert_eq!(
            decode_path("metadata.annotations.kubectl%2Ekubernetes%2Eio/restartedAt"),
            "metadata.annotations.kubectl.kubernetes.io/restartedAt"
        );
    }

    #[test]
    fn flatten_and_reconstruct_roundtrip_with_dotted_keys() {
        let root = json!({
            "metadata": {
                "name": "worker-a",
                "annotations": {
                    "kubectl.kubernetes.io/restartedAt": "2026-02-22T10:00:00Z",
                    "x%y": "pct"
                }
            }
        });

        let fields = flatten_json_to_fields(&root);
        assert_eq!(
            fields.get("metadata.annotations.kubectl%2Ekubernetes%2Eio/restartedAt"),
            Some(&Value::String("2026-02-22T10:00:00Z".to_string()))
        );
        assert_eq!(
            fields.get("metadata.annotations.x%25y"),
            Some(&Value::String("pct".to_string()))
        );

        let reconstructed = reconstruct_nested_from_fields(&fields);
        assert_eq!(reconstructed, root);
    }

    #[test]
    fn select_parent_path_rebuilds_with_dotted_keys() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "metadata.annotations.kubectl%2Ekubernetes%2Eio/restartedAt".to_string(),
            Value::String("2026-02-22T10:00:00Z".to_string()),
        );
        fields.insert(
            "metadata.annotations.app%2Ekubernetes%2Eio/name".to_string(),
            Value::String("api".to_string()),
        );

        let selected = select_path_value(&fields, "metadata.annotations")
            .expect("metadata.annotations must be reconstructed");
        assert_eq!(
            selected,
            json!({
                "kubectl.kubernetes.io/restartedAt": "2026-02-22T10:00:00Z",
                "app.kubernetes.io/name": "api"
            })
        );
    }

    #[test]
    fn preserves_distinct_percent_encoded_keys() {
        let root = json!({
            "metadata": {
                "labels": {
                    "a.b": "dot",
                    "a%2Eb": "literal"
                }
            }
        });

        let fields = flatten_json_to_fields(&root);
        assert_eq!(
            fields.get("metadata.labels.a%2Eb"),
            Some(&Value::String("dot".to_string()))
        );
        assert_eq!(
            fields.get("metadata.labels.a%252Eb"),
            Some(&Value::String("literal".to_string()))
        );
        assert_eq!(reconstruct_nested_from_fields(&fields), root);
    }
}
