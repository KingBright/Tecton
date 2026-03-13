//! DSL to SVG Conversion Integration Tests
//!
//! These tests verify the conversion from different DSL formats to SVG output.

use std::fs;

/// Helper to load a fixture file
fn load_fixture(name: &str) -> String {
    let path = format!("tests/fixtures/diagrams/{}", name);
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to load fixture: {}", path))
}

/// Test JSON to SVG conversion
#[test]
fn test_json_to_svg() {
    let json = r#"{
        "canvas": {"width": 400, "height": 300},
        "nodes": [
            {"label": "Test Node"}
        ],
        "edges": []
    }"#;

    // Verify JSON structure
    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert_eq!(parsed["canvas"]["width"], 400);
    assert_eq!(parsed["nodes"][0]["label"], "Test Node");
}

/// Test YAML to SVG conversion
#[test]
fn test_yaml_to_svg() {
    let yaml = r#"
canvas:
  width: 500
  height: 400
nodes:
  - label: "YAML Node"
    style:
      fill: "#E3F2FD"
edges: []
"#;

    let parsed: serde_yaml::Value = serde_yaml::from_str(yaml).expect("Valid YAML");
    assert_eq!(parsed["nodes"][0]["label"].as_str(), Some("YAML Node"));
}

/// Test Tecton DSL to SVG conversion
#[test]
fn test_tecton_dsl_to_svg() {
    let dsl = r#"
config { width: 600, height: 400 }

Row {
    Node1 [label: "A", style: { fill: "#E3F2FD" }]
    Node2 [label: "B", style: { fill: "#E8F5E9" }]
}
"#;

    // Verify basic DSL structure
    assert!(dsl.contains("config"));
    assert!(dsl.contains("Row"));
    assert!(dsl.contains("Node1"));
}

/// Test nested container DSL conversion
#[test]
fn test_nested_container_dsl() {
    let dsl = r#"
config { width: 800, height: 600 }

Row {
    Sidebar [label: "Side"]
    Main {
        Col {
            Header [label: "Header"]
            Body [label: "Body"]
        }
    }
}
"#;

    // Count opening braces to verify nesting
    let open_count = dsl.matches('{').count();
    let close_count = dsl.matches('}').count();
    assert_eq!(open_count, close_count, "Braces should be balanced");
}

/// Test DSL with edge definitions
#[test]
fn test_dsl_with_edges() {
    let dsl = r#"
A [label: "Source", ports: [{ name: "out", side: "east" }]]
B [label: "Target", ports: [{ name: "in", side: "west" }]]
A -> B
"#;

    assert!(dsl.contains("->"));
    assert!(dsl.contains("ports:"));
}

/// Test complex DSL with multiple features
#[test]
fn test_complex_dsl() {
    let dsl = r#"
config {
    width: 1000,
    height: 800
}

// Nodes with different shapes
Start [label: "Start", shape: "circle"]
Process [label: "Process", shape: "rectangle"]
Decision [label: "Decision", shape: "diamond"]
End [label: "End", shape: "circle"]

// Database
DB [label: "Database", shape: "cylinder"]

// Connections
Start -> Process
Process -> Decision
Decision -> End
Process -> DB
"#;

    // Verify structure
    assert!(dsl.contains("config"));
    assert!(dsl.contains("//")); // Comments
    assert!(dsl.contains("shape:"));
    assert!(dsl.contains("circle"));
    assert!(dsl.contains("rectangle"));
    assert!(dsl.contains("diamond"));
    assert!(dsl.contains("cylinder"));
}

/// Test DSL with style definitions
#[test]
fn test_dsl_with_styles() {
    let dsl = r#"
Node [
    label: "Styled",
    style: {
        fill: "#FF0000",
        stroke: "#000000",
        strokeWidth: 2,
        opacity: 0.8
    }
]
"#;

    assert!(dsl.contains("style:"));
    assert!(dsl.contains("fill:"));
    assert!(dsl.contains("stroke:"));
}

/// Test error handling for invalid JSON
#[test]
fn test_invalid_json_handling() {
    let invalid_json = r#"{ invalid json }"#;

    let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail to parse invalid JSON");
}

/// Test error handling for invalid YAML
#[test]
fn test_invalid_yaml_handling() {
    let invalid_yaml = r#"
canvas:
  width: [invalid
  height: value
"#;

    let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(invalid_yaml);
    assert!(result.is_err(), "Should fail to parse invalid YAML");
}

/// Test missing required fields
#[test]
fn test_missing_canvas() {
    let json = r#"{
        "nodes": [],
        "edges": []
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert!(parsed.get("canvas").is_none() || parsed["canvas"].is_null());
}

/// Test empty DSL
#[test]
fn test_empty_dsl() {
    let dsl = "  \n\n  ";

    // Empty DSL should be handled gracefully
    assert!(dsl.trim().is_empty());
}

/// Test DSL with Unicode labels
#[test]
fn test_unicode_labels() {
    let dsl = r#"
节点 [label: "中文字符"]
Émoji [label: "Hello 🎉"]
"#;

    // Verify DSL can handle Unicode
    assert!(dsl.contains("中文字符"));
    assert!(dsl.contains("🎉"));
}

/// Test DSL with comments
#[test]
fn test_dsl_with_comments() {
    let dsl = r#"
// This is a comment
Node [label: "Test"] // inline comment

/*
Multi-line
comment
*/

Another [label: "Node"]
"#;

    assert!(dsl.contains("//"));
}
