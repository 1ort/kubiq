use std::collections::BTreeSet;

use crate::dynamic_object::DynamicObject;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailLevel {
    Summary,
    Describe,
}

pub fn print(
    objects: &[DynamicObject],
    format: OutputFormat,
    detail: DetailLevel,
    select_paths: Option<&[String]>,
) -> Result<(), String> {
    let content = match format {
        OutputFormat::Table => render_table(objects, detail, select_paths),
        OutputFormat::Json => render_json(objects, detail, select_paths)?,
    };
    println!("{content}");
    Ok(())
}

pub fn render_json(
    objects: &[DynamicObject],
    detail: DetailLevel,
    select_paths: Option<&[String]>,
) -> Result<String, String> {
    let rows: Vec<_> = objects
        .iter()
        .map(|object| project_fields(object, detail, select_paths))
        .collect();
    serde_json::to_string_pretty(&rows)
        .map_err(|error| format!("failed to serialize json output: {error}"))
}

pub fn render_table(
    objects: &[DynamicObject],
    detail: DetailLevel,
    select_paths: Option<&[String]>,
) -> String {
    let projected: Vec<_> = objects
        .iter()
        .map(|object| project_fields(object, detail, select_paths))
        .collect();
    let columns = collect_columns(&projected);
    if columns.is_empty() {
        return "items: 0".to_string();
    }

    let widths = compute_widths(&projected, &columns);
    let mut lines = Vec::new();
    lines.push(format_row(&columns, &widths));
    lines.push(format_separator(&widths));

    for fields in projected {
        let row: Vec<String> = columns
            .iter()
            .map(|column| {
                fields
                    .get(column)
                    .map(value_to_cell)
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect();
        lines.push(format_row(&row, &widths));
    }

    lines.push(format!("items: {}", objects.len()));
    lines.join("\n")
}

fn project_fields(
    object: &DynamicObject,
    detail: DetailLevel,
    select_paths: Option<&[String]>,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    if let Some(select_paths) = select_paths {
        let mut projected = std::collections::BTreeMap::new();
        for path in select_paths {
            let value = select_value(object, path).unwrap_or(serde_json::Value::Null);
            projected.insert(path.clone(), value);
        }
        return projected;
    }

    match detail {
        DetailLevel::Describe => object.fields.clone(),
        DetailLevel::Summary => {
            let mut projected = std::collections::BTreeMap::new();
            let name = object
                .fields
                .get("metadata.name")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::String("-".to_string()));
            projected.insert("name".to_string(), name);
            projected
        }
    }
}

fn select_value(
    object: &DynamicObject,
    path: &str,
) -> Option<serde_json::Value> {
    if let Some(value) = object.fields.get(path) {
        return Some(value.clone());
    }

    let prefix = format!("{path}.");
    let mut nested = serde_json::Value::Object(serde_json::Map::new());
    let mut found = false;

    for (key, value) in &object.fields {
        if let Some(suffix) = key.strip_prefix(&prefix) {
            if suffix.is_empty() {
                continue;
            }
            found = true;
            let parts: Vec<&str> = suffix.split('.').collect();
            insert_nested_value(&mut nested, &parts, value.clone());
        }
    }

    if found { Some(nested) } else { None }
}

fn insert_nested_value(
    node: &mut serde_json::Value,
    parts: &[&str],
    value: serde_json::Value,
) {
    if parts.is_empty() {
        *node = value;
        return;
    }

    if let Ok(index) = parts[0].parse::<usize>() {
        if !node.is_array() {
            *node = serde_json::Value::Array(Vec::new());
        }
        if let serde_json::Value::Array(array) = node {
            while array.len() <= index {
                array.push(serde_json::Value::Null);
            }
            insert_nested_value(&mut array[index], &parts[1..], value);
        }
        return;
    }

    if !node.is_object() {
        *node = serde_json::Value::Object(serde_json::Map::new());
    }
    if let serde_json::Value::Object(map) = node {
        let entry = map
            .entry(parts[0].to_string())
            .or_insert(serde_json::Value::Null);
        insert_nested_value(entry, &parts[1..], value);
    }
}

fn collect_columns(
    objects: &[std::collections::BTreeMap<String, serde_json::Value>]
) -> Vec<String> {
    let mut set = BTreeSet::new();
    for fields in objects {
        for key in fields.keys() {
            set.insert(key.clone());
        }
    }
    set.into_iter().collect()
}

fn compute_widths(
    objects: &[std::collections::BTreeMap<String, serde_json::Value>],
    columns: &[String],
) -> Vec<usize> {
    columns
        .iter()
        .map(|column| {
            let mut width = column.len();
            for fields in objects {
                let cell = fields
                    .get(column)
                    .map(value_to_cell)
                    .unwrap_or_else(|| "-".to_string());
                width = width.max(cell.len());
            }
            width
        })
        .collect()
}

fn value_to_cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

fn format_row(
    cells: &[String],
    widths: &[usize],
) -> String {
    let mut out = String::new();
    out.push('|');
    for (index, cell) in cells.iter().enumerate() {
        out.push(' ');
        out.push_str(cell);
        let padding = widths[index].saturating_sub(cell.len());
        for _ in 0..padding {
            out.push(' ');
        }
        out.push(' ');
        out.push('|');
    }
    out
}

fn format_separator(widths: &[usize]) -> String {
    let mut out = String::new();
    out.push('|');
    for width in widths {
        out.push(' ');
        for _ in 0..*width {
            out.push('-');
        }
        out.push(' ');
        out.push('|');
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::Value;

    use crate::dynamic_object::DynamicObject;

    use super::{DetailLevel, render_json, render_table};

    #[test]
    fn renders_table_with_columns_and_count() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "metadata.name".to_string(),
            Value::String("pod-a".to_string()),
        );
        fields.insert(
            "metadata.namespace".to_string(),
            Value::String("demo-a".to_string()),
        );
        let out = render_table(&[DynamicObject { fields }], DetailLevel::Describe, None);

        assert!(out.contains("metadata.name"));
        assert!(out.contains("metadata.namespace"));
        assert!(out.contains("pod-a"));
        assert!(out.contains("items: 1"));
    }

    #[test]
    fn renders_json_array() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "metadata.name".to_string(),
            Value::String("pod-a".to_string()),
        );
        let out = render_json(&[DynamicObject { fields }], DetailLevel::Describe, None)
            .expect("json output must serialize");

        assert!(out.starts_with("["));
        assert!(out.contains("\"metadata.name\": \"pod-a\""));
    }

    #[test]
    fn renders_summary_with_name_only() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "metadata.name".to_string(),
            Value::String("pod-a".to_string()),
        );
        fields.insert(
            "metadata.namespace".to_string(),
            Value::String("demo-a".to_string()),
        );

        let table = render_table(
            &[DynamicObject {
                fields: fields.clone(),
            }],
            DetailLevel::Summary,
            None,
        );
        assert!(table.contains("| name"));
        assert!(table.contains("pod-a"));
        assert!(!table.contains("metadata.namespace"));

        let json = render_json(&[DynamicObject { fields }], DetailLevel::Summary, None)
            .expect("json output must serialize");
        assert!(json.contains("\"name\": \"pod-a\""));
        assert!(!json.contains("metadata.namespace"));
    }

    #[test]
    fn select_projection_overrides_summary() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "metadata.name".to_string(),
            Value::String("pod-a".to_string()),
        );
        fields.insert(
            "metadata.namespace".to_string(),
            Value::String("demo-a".to_string()),
        );

        let select = vec!["metadata.namespace".to_string()];
        let table = render_table(
            &[DynamicObject { fields }],
            DetailLevel::Summary,
            Some(&select),
        );
        assert!(table.contains("metadata.namespace"));
        assert!(table.contains("demo-a"));
        assert!(!table.contains("| name"));
    }

    #[test]
    fn select_parent_path_rebuilds_nested_json() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "metadata.name".to_string(),
            Value::String("pod-a".to_string()),
        );
        fields.insert(
            "metadata.namespace".to_string(),
            Value::String("demo-a".to_string()),
        );

        let select = vec!["metadata".to_string()];
        let json = render_json(
            &[DynamicObject { fields }],
            DetailLevel::Summary,
            Some(&select),
        )
        .expect("json output must serialize");

        assert!(json.contains("\"metadata\": {"));
        assert!(json.contains("\"name\": \"pod-a\""));
        assert!(json.contains("\"namespace\": \"demo-a\""));
    }
}
