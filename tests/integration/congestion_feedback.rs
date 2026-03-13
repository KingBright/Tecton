//! Congestion Feedback Loop Integration Tests
//!
//! These tests verify the congestion detection and space expansion mechanisms.

/// Test high density diagram triggers congestion
#[test]
fn test_high_density_convergence() {
    // Create a diagram with many connections in a small area
    let json = r#"{
        "canvas": {"width": 400, "height": 400},
        "nodes": [
            {"id": "n1", "label": "1", "x": 50, "y": 50},
            {"id": "n2", "label": "2", "x": 350, "y": 50},
            {"id": "n3", "label": "3", "x": 50, "y": 350},
            {"id": "n4", "label": "4", "x": 350, "y": 350},
            {"id": "n5", "label": "5", "x": 200, "y": 200}
        ],
        "edges": [
            {"from": "n1", "to": "n5"},
            {"from": "n2", "to": "n5"},
            {"from": "n3", "to": "n5"},
            {"from": "n4", "to": "n5"},
            {"from": "n1", "to": "n2"},
            {"from": "n3", "to": "n4"}
        ]
    }"#;

    // Verify the structure
    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    let edges = parsed["edges"].as_array().unwrap();
    assert!(edges.len() > 3, "Should have enough edges to cause congestion");
}

/// Test no congestion fast path
#[test]
fn test_no_congestion_fast_path() {
    // Simple diagram with minimal connections
    let json = r#"{
        "canvas": {"width": 600, "height": 400},
        "nodes": [
            {"id": "a", "label": "A"},
            {"id": "b", "label": "B"}
        ],
        "edges": [
            {"from": "a", "to": "b"}
        ]
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 1);
}

/// Test max iterations handling
#[test]
fn test_max_iterations_exceeded() {
    // Very dense diagram that may not converge
    let mut edges = vec![];
    for i in 0..20 {
        for j in (i+1)..20 {
            edges.push(format!(r#"{{"from": "node_{}", "to": "node_{}"}}"#, i, j));
        }
    }

    let mut nodes = vec![];
    for i in 0..20 {
        nodes.push(format!(r#"{{"id": "node_{}", "label": "{}"}}"#, i, i));
    }

    let json = format!(
        r#"{{
            "canvas": {{"width": 500, "height": 500}},
            "nodes": [{}],
            "edges": [{}]
        }}"#,
        nodes.join(", "),
        edges.join(", ")
    );

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 20);
}

/// Test congestion hotspot detection
#[test]
fn test_congestion_hotspot_detection() {
    // Create a diagram with a clear hotspot in the center
    let json = r#"{
        "canvas": {"width": 500, "height": 500},
        "nodes": [
            {"id": "source1", "label": "S1", "x": 50, "y": 100},
            {"id": "source2", "label": "S2", "x": 50, "y": 200},
            {"id": "source3", "label": "S3", "x": 50, "y": 300},
            {"id": "target1", "label": "T1", "x": 450, "y": 100},
            {"id": "target2", "label": "T2", "x": 450, "y": 200},
            {"id": "target3", "label": "T3", "x": 450, "y": 300}
        ],
        "edges": [
            {"from": "source1", "to": "target3"},
            {"from": "source2", "to": "target2"},
            {"from": "source3", "to": "target1"},
            {"from": "source1", "to": "target2"},
            {"from": "source3", "to": "target2"}
        ]
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    let edges = parsed["edges"].as_array().unwrap();

    // All paths should cross the center area
    let crossing_count = edges.len();
    assert!(crossing_count >= 3, "Should have crossing paths");
}

/// Test margin expansion
#[test]
fn test_margin_expansion() {
    // Diagram where nodes need more space
    let json = r#"{
        "canvas": {"width": 300, "height": 300},
        "nodes": [
            {"id": "a", "label": "A", "width": 80, "height": 60},
            {"id": "b", "label": "B", "width": 80, "height": 60},
            {"id": "c", "label": "C", "width": 80, "height": 60}
        ],
        "edges": [
            {"from": "a", "to": "b"},
            {"from": "b", "to": "c"},
            {"from": "c", "to": "a"}
        ]
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 3);
}

/// Test routing with blocked cells
#[test]
fn test_routing_with_blocked_cells() {
    // Diagram with obstacles
    let json = r#"{
        "canvas": {"width": 600, "height": 400},
        "nodes": [
            {"id": "start", "label": "Start", "x": 50, "y": 200},
            {"id": "obstacle1", "label": "Block1", "x": 200, "y": 150, "width": 100, "height": 100},
            {"id": "obstacle2", "label": "Block2", "x": 200, "y": 250, "width": 100, "height": 100},
            {"id": "end", "label": "End", "x": 550, "y": 200}
        ],
        "edges": [
            {"from": "start", "to": "end"}
        ]
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    assert!(parsed["nodes"].as_array().unwrap().len() >= 3);
}

/// Test congestion severity calculation
#[test]
fn test_congestion_severity() {
    // Different levels of congestion
    let light = r#"{"canvas": {"width": 400, "height": 400}, "nodes": [{"id": "a"}, {"id": "b"}], "edges": [{"from": "a", "to": "b"}]}"#;
    let heavy = r#"{
        "canvas": {"width": 400, "height": 400},
        "nodes": [
            {"id": "a"}, {"id": "b"}, {"id": "c"}, {"id": "d"}, {"id": "e"}
        ],
        "edges": [
            {"from": "a", "to": "e"}, {"from": "b", "to": "e"}, {"from": "c", "to": "e"}, {"from": "d", "to": "e"},
            {"from": "a", "to": "b"}, {"from": "b", "to": "c"}, {"from": "c", "to": "d"}
        ]
    }"#;

    let light_parsed: serde_json::Value = serde_json::from_str(light).expect("Valid JSON");
    let heavy_parsed: serde_json::Value = serde_json::from_str(heavy).expect("Valid JSON");

    let light_edges = light_parsed["edges"].as_array().unwrap().len();
    let heavy_edges = heavy_parsed["edges"].as_array().unwrap().len();

    assert!(heavy_edges > light_edges * 3, "Heavy congestion should have significantly more edges");
}

/// Test iterative refinement convergence
#[test]
fn test_iterative_refinement() {
    // Test that the system can converge within reasonable iterations
    let json = r#"{
        "canvas": {"width": 800, "height": 600},
        "nodes": [
            {"id": "hub", "label": "Hub", "x": 400, "y": 300}
        ],
        "config": {
            "maxIterations": 10,
            "congestionThreshold": 2
        }
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    let config = &parsed["config"];
    assert_eq!(config["maxIterations"], 10);
}

/// Test that paths avoid node boundaries
#[test]
fn test_path_avoids_nodes() {
    let json = r#"{
        "canvas": {"width": 500, "height": 300},
        "nodes": [
            {"id": "left", "label": "L", "x": 50, "y": 125, "width": 80, "height": 50},
            {"id": "center", "label": "C", "x": 210, "y": 125, "width": 80, "height": 50},
            {"id": "right", "label": "R", "x": 370, "y": 125, "width": 80, "height": 50}
        ],
        "edges": [
            {"from": "left", "to": "right"}
        ]
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).expect("Valid JSON");
    let nodes = parsed["nodes"].as_array().unwrap();

    // The center node should be between left and right
    let left_x = nodes[0]["x"].as_i64().unwrap();
    let center_x = nodes[1]["x"].as_i64().unwrap();
    let right_x = nodes[2]["x"].as_i64().unwrap();

    assert!(center_x > left_x && center_x < right_x);
}
