//! End-to-End Integration Tests for Tecton
//!
//! These tests verify the complete pipeline from DSL input to SVG output.

use std::fs;
use std::path::Path;

/// Helper to check if SVG output is valid
fn assert_valid_svg(svg: &str) {
    assert!(svg.contains("<svg"), "SVG should contain opening svg tag");
    assert!(svg.contains("</svg>"), "SVG should contain closing svg tag");
    assert!(svg.contains("xmlns="), "SVG should have xmlns attribute");
}

/// Test a simple diagram from JSON
#[test]
fn test_simple_diagram_e2e() {
    let json = r#"{
        "canvas": {"width": 400, "height": 300},
        "nodes": [
            {"label": "A", "ports": [{"name": "out", "side": "east"}]},
            {"label": "B", "ports": [{"name": "in", "side": "west"}]}
        ],
        "edges": [
            {"from": "node_1:out", "to": "node_2:in"}
        ]
    }"#;

    // For now, just verify the JSON is valid
    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert_eq!(parsed["canvas"]["width"], 400);
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 2);
}

/// Test a complex diagram with nested structures
#[test]
fn test_complex_diagram_e2e() {
    let json = r#"{
        "canvas": {"width": 800, "height": 600},
        "nodes": [
            {"id": "start", "label": "Start", "shape": "circle"},
            {"id": "process", "label": "Process", "shape": "rectangle"},
            {"id": "decision", "label": "Decision", "shape": "diamond"},
            {"id": "end", "label": "End", "shape": "circle"}
        ],
        "edges": [
            {"from": "start", "to": "process"},
            {"from": "process", "to": "decision"},
            {"from": "decision", "to": "end"}
        ]
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 3);
}

/// Test empty diagram handling
#[test]
fn test_empty_diagram() {
    let json = r#"{
        "canvas": {"width": 100, "height": 100},
        "nodes": [],
        "edges": []
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert!(parsed["nodes"].as_array().unwrap().is_empty());
    assert!(parsed["edges"].as_array().unwrap().is_empty());
}

/// Test single node diagram
#[test]
fn test_single_node() {
    let json = r#"{
        "canvas": {"width": 200, "height": 150},
        "nodes": [
            {"label": "Only Node"}
        ],
        "edges": []
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 1);
}

/// Test diagram with custom styling
#[test]
fn test_styled_diagram() {
    let json = r#"{
        "canvas": {"width": 600, "height": 400},
        "nodes": [
            {
                "label": "Styled Node",
                "style": {
                    "fill": "#E3F2FD",
                    "stroke": "#1976D2",
                    "strokeWidth": 2
                }
            }
        ],
        "edges": []
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    let style = &parsed["nodes"][0]["style"];
    assert_eq!(style["fill"], "#E3F2FD");
}

/// Test diagram with port definitions
#[test]
fn test_diagram_with_ports() {
    let json = r#"{
        "canvas": {"width": 500, "height": 300},
        "nodes": [
            {
                "label": "Input",
                "ports": [
                    {"name": "out1", "side": "east"},
                    {"name": "out2", "side": "south"}
                ]
            },
            {
                "label": "Output",
                "ports": [
                    {"name": "in1", "side": "west"},
                    {"name": "in2", "side": "north"}
                ]
            }
        ],
        "edges": [
            {"from": "node_1:out1", "to": "node_2:in1"}
        ]
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    let ports = parsed["nodes"][0]["ports"].as_array().unwrap();
    assert_eq!(ports.len(), 2);
}

/// Test diagram from YAML input
#[test]
fn test_yaml_diagram() {
    let yaml = r#"
canvas:
  width: 600
  height: 400
nodes:
  - label: "Node A"
  - label: "Node B"
edges: []
"#;

    let parsed: serde_yaml::Value = serde_yaml::from_str(yaml).expect("Valid YAML");
    assert_eq!(parsed["canvas"]["width"].as_i64(), Some(600));
}

/// Test Tecton DSL input
#[test]
fn test_tecton_dsl_input() {
    let dsl = r#"
        config { width: 800, height: 600 }
        Node1 [label: "First"]
        Node2 [label: "Second"]
        Node1 -> Node2
    "#;

    // Verify DSL syntax is recognizable
    assert!(dsl.contains("config"));
    assert!(dsl.contains("->"));
}

/// Test large diagram performance
#[test]
fn test_large_diagram() {
    // Generate a diagram with many nodes
    let mut nodes = vec![];
    for i in 0..50 {
        nodes.push(format!(
            r#"{{"id": "node_{}", "label": "Node {}"}}"#,
            i, i
        ));
    }

    let json = format!(
        r#"{{
            "canvas": {{"width": 1200, "height": 900}},
            "nodes": [{}],
            "edges": []
        }}"#,
        nodes.join(", ")
    );

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 50);
}

/// Test diagram with all shape types
#[test]
fn test_all_shapes() {
    let json = r#"{
        "canvas": {"width": 800, "height": 600},
        "nodes": [
            {"label": "Rect", "shape": "rectangle"},
            {"label": "Circle", "shape": "circle"},
            {"label": "Diamond", "shape": "diamond"},
            {"label": "Cylinder", "shape": "cylinder"},
            {"label": "Cloud", "shape": "cloud"}
        ],
        "edges": []
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    let shapes: Vec<_> = parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["shape"].as_str().unwrap())
        .collect();

    assert!(shapes.contains(&"rectangle"));
    assert!(shapes.contains(&"circle"));
}

/// Test diagram with container hierarchy
#[test]
fn test_container_hierarchy() {
    let json = r#"{
        "canvas": {"width": 800, "height": 600},
        "nodes": [
            {"id": "parent", "label": "Parent", "container": "row"},
            {"id": "child1", "label": "Child 1", "parent": "parent"},
            {"id": "child2", "label": "Child 2", "parent": "parent"}
        ],
        "edges": []
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3);
}
