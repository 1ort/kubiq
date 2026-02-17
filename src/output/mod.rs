use std::collections::BTreeSet;

use crate::dynamic_object::DynamicObject;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
}

pub fn print(
    objects: &[DynamicObject],
    format: OutputFormat,
) -> Result<(), String> {
    let content = match format {
        OutputFormat::Table => render_table(objects),
        OutputFormat::Json => render_json(objects)?,
    };
    println!("{content}");
    Ok(())
}

pub fn render_json(objects: &[DynamicObject]) -> Result<String, String> {
    let rows: Vec<_> = objects.iter().map(|object| &object.fields).collect();
    serde_json::to_string_pretty(&rows)
        .map_err(|error| format!("failed to serialize json output: {error}"))
}

pub fn render_table(objects: &[DynamicObject]) -> String {
    let columns = collect_columns(objects);
    if columns.is_empty() {
        return "items: 0".to_string();
    }

    let widths = compute_widths(objects, &columns);
    let mut lines = Vec::new();
    lines.push(format_row(&columns, &widths));
    lines.push(format_separator(&widths));

    for object in objects {
        let row: Vec<String> = columns
            .iter()
            .map(|column| {
                object
                    .fields
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

fn collect_columns(objects: &[DynamicObject]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for object in objects {
        for key in object.fields.keys() {
            set.insert(key.clone());
        }
    }
    set.into_iter().collect()
}

fn compute_widths(
    objects: &[DynamicObject],
    columns: &[String],
) -> Vec<usize> {
    columns
        .iter()
        .map(|column| {
            let mut width = column.len();
            for object in objects {
                let cell = object
                    .fields
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

    use super::{render_json, render_table};

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
        let out = render_table(&[DynamicObject { fields }]);

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
        let out = render_json(&[DynamicObject { fields }]).expect("json output must serialize");

        assert!(out.starts_with("["));
        assert!(out.contains("\"metadata.name\": \"pod-a\""));
    }
}
