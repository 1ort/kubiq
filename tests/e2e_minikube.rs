use std::process::Command;

use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

fn e2e_enabled() -> bool {
    std::env::var("KUBIQ_E2E").as_deref() == Ok("1")
}

fn cluster_ready() -> bool {
    let output = Command::new("kubectl")
        .args(["get", "ns", "demo-a", "-o", "name"])
        .output();

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn run_kubiq(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kubiq"));
    cmd.args(args);
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("http_proxy");
    cmd.env_remove("https_proxy");
    cmd.env_remove("all_proxy");
    cmd.output().expect("kubiq command must run")
}

#[test]
fn e2e_table_where_select_for_core_resource() {
    if !e2e_enabled() || !cluster_ready() {
        return;
    }

    let output = run_kubiq(&[
        "pods",
        "where",
        "metadata.namespace",
        "==",
        "demo-a",
        "select",
        "metadata.name,metadata.namespace",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout must be valid UTF-8");
    assert!(stdout.contains("metadata.name"));
    assert!(stdout.contains("metadata.namespace"));
    assert!(stdout.contains("demo-a"));
}

#[test]
fn e2e_table_order_by_for_core_resource() {
    if !e2e_enabled() || !cluster_ready() {
        return;
    }

    let output = run_kubiq(&[
        "pods",
        "where",
        "metadata.namespace",
        "==",
        "demo-a",
        "-o",
        "json",
        "order",
        "by",
        "metadata.name",
        "desc",
        "select",
        "metadata.name",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rows: JsonValue =
        serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    let items = rows.as_array().expect("must return array");
    assert!(items.len() >= 2, "expected at least two pods in demo-a");

    let names: Vec<String> = items
        .iter()
        .filter_map(|row| {
            row.get("metadata.name")
                .and_then(JsonValue::as_str)
                .map(ToString::to_string)
        })
        .collect();
    assert_eq!(names.len(), items.len(), "every row must have metadata.name");

    let mut expected = names.clone();
    expected.sort_by(|left, right| right.cmp(left));
    assert_eq!(names, expected);
}

#[test]
fn e2e_json_select_parent_path_is_nested() {
    if !e2e_enabled() || !cluster_ready() {
        return;
    }

    let output = run_kubiq(&[
        "pods",
        "where",
        "metadata.name",
        "==",
        "worker-a",
        "-o",
        "json",
        "select",
        "metadata",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rows: JsonValue =
        serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    let first = rows
        .as_array()
        .and_then(|items| items.first())
        .expect("must return at least one object");

    let metadata = first
        .get("metadata")
        .and_then(JsonValue::as_object)
        .expect("metadata must be nested object");

    assert_eq!(
        metadata.get("name"),
        Some(&JsonValue::String("worker-a".to_string()))
    );
    assert_eq!(
        metadata.get("namespace"),
        Some(&JsonValue::String("demo-a".to_string()))
    );
}

#[test]
fn e2e_json_order_by_for_crd_widget() {
    if !e2e_enabled() || !cluster_ready() {
        return;
    }

    let output = run_kubiq(&[
        "widgets",
        "where",
        "metadata.name",
        "!=",
        "widget-z",
        "-o",
        "json",
        "order",
        "by",
        "spec.owner",
        "desc",
        ",",
        "metadata.name",
        "asc",
        "select",
        "metadata.name,spec.owner",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rows: JsonValue =
        serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    let items = rows.as_array().expect("must return array");
    assert!(items.len() >= 2, "expected at least two widgets");

    let mut previous: Option<(String, String)> = None;
    for row in items {
        let owner = row
            .get("spec.owner")
            .and_then(JsonValue::as_str)
            .expect("spec.owner must be present")
            .to_string();
        let name = row
            .get("metadata.name")
            .and_then(JsonValue::as_str)
            .expect("metadata.name must be present")
            .to_string();

        if let Some((prev_owner, prev_name)) = previous.as_ref() {
            if owner == *prev_owner {
                assert!(name >= *prev_name, "metadata.name must be asc within same owner");
            } else {
                assert!(owner <= *prev_owner, "spec.owner must be desc");
            }
        }
        previous = Some((owner, name));
    }
}

#[test]
fn e2e_yaml_describe_is_nested() {
    if !e2e_enabled() || !cluster_ready() {
        return;
    }

    let output = run_kubiq(&[
        "pods",
        "where",
        "metadata.name",
        "==",
        "worker-a",
        "-o",
        "yaml",
        "-d",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rows: YamlValue =
        serde_yaml::from_slice(&output.stdout).expect("stdout must be valid YAML");
    let first = rows
        .as_sequence()
        .and_then(|items| items.first())
        .expect("must return at least one object");

    let metadata = first
        .get("metadata")
        .and_then(YamlValue::as_mapping)
        .expect("metadata must be nested mapping");

    let name_key = YamlValue::String("name".to_string());
    let namespace_key = YamlValue::String("namespace".to_string());
    assert_eq!(
        metadata.get(&name_key),
        Some(&YamlValue::String("worker-a".to_string()))
    );
    assert_eq!(
        metadata.get(&namespace_key),
        Some(&YamlValue::String("demo-a".to_string()))
    );
}

#[test]
fn e2e_select_for_crd_widget() {
    if !e2e_enabled() || !cluster_ready() {
        return;
    }

    let output = run_kubiq(&[
        "widgets",
        "where",
        "spec.enabled",
        "==",
        "true",
        "-o",
        "json",
        "select",
        "metadata.name,spec.owner",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rows: JsonValue =
        serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    let first = rows
        .as_array()
        .and_then(|items| items.first())
        .expect("must return at least one object");

    assert_eq!(
        first.get("metadata.name"),
        Some(&JsonValue::String("widget-a".to_string()))
    );
    assert_eq!(
        first.get("spec.owner"),
        Some(&JsonValue::String("team-a".to_string()))
    );
}
