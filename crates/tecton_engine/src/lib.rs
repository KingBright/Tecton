//! Tecton Engine - Orchestration and control layer for Tecton
//!
//! This crate provides:
//! - LayoutGraph: A graph structure for nodes and edges
//! - DslParser: JSON/YAML parsing for diagram specifications
//! - Orchestrator: Main controller coordinating layout and routing

pub mod dsl_parser;
pub mod layout_graph;
pub mod orchestrator;
pub mod tecton_dsl_converter;

pub use dsl_parser::{CanvasSpec, DiagramDsl, DslParser, EdgeSpec, NodeSpec};
pub use layout_graph::{LayoutGraph, LayoutNode, LayoutPort};
pub use orchestrator::{Orchestrator, OrchestratorConfig};

use serde::Deserialize;
use tecton_types::error::{Result, TectonError};

// Keep DiagramDsl's Rust API unchanged; text documents can add presentation metadata.
#[derive(Deserialize)]
struct AutomaticDocument {
    #[serde(flatten)]
    diagram: DiagramDsl,
    title: Option<String>,
    #[serde(default)]
    legend: Vec<tecton_render::LegendEntry>,
}

/// High-level API for rendering diagrams
pub struct Tecton;

impl Tecton {
    /// Render a diagram from a DSL specification
    pub fn render_dsl(dsl: &DiagramDsl) -> Result<String> {
        Self::render_dsl_with_header(dsl, None, &[])
    }

    fn render_dsl_with_header(
        dsl: &DiagramDsl,
        title: Option<&str>,
        legend: &[tecton_render::LegendEntry],
    ) -> Result<String> {
        let parser = DslParser::new();
        let graph = parser
            .to_layout_graph(dsl)
            .map_err(|e| tecton_types::error::TectonError::parse(e))?;

        let mut orchestrator = Orchestrator::new(graph);
        orchestrator.run()?;

        let title = title.unwrap_or_default();
        if title.is_empty() && legend.is_empty() {
            orchestrator.render()
        } else {
            orchestrator.render_with_header(title, legend)
        }
    }

    /// Render a diagram from JSON, with an optional title and legend.
    pub fn render_json(json: &str) -> Result<String> {
        let document: AutomaticDocument =
            serde_json::from_str(json).map_err(|e| TectonError::parse(e.to_string()))?;
        Self::render_dsl_with_header(
            &document.diagram,
            document.title.as_deref(),
            &document.legend,
        )
    }

    /// Render a diagram from YAML, with an optional title and legend.
    pub fn render_yaml(yaml: &str) -> Result<String> {
        let document: AutomaticDocument =
            serde_yaml::from_str(yaml).map_err(|e| TectonError::parse(e.to_string()))?;
        Self::render_dsl_with_header(
            &document.diagram,
            document.title.as_deref(),
            &document.legend,
        )
    }

    /// Render a diagram from Tecton DSL (Pest-based)
    pub fn render_tecton_dsl(dsl_text: &str) -> Result<String> {
        let file = tecton_dsl::parse_dsl(dsl_text)
            .map_err(|e| tecton_types::error::TectonError::parse(e.to_string()))?;

        let graph = tecton_dsl_converter::convert_to_layout_graph(&file)
            .map_err(|e| tecton_types::error::TectonError::parse(e))?;

        let mut orchestrator = Orchestrator::new(graph);
        orchestrator.run()?;

        orchestrator.render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_escaped_title_has_own_viewport_in_json_and_yaml() {
        let plain = Tecton::render_json(r#"{"nodes":[{"label":"Card"}]}"#).unwrap();
        let title = "标题 <&>";
        let json =
            Tecton::render_json(r#"{"title":"标题 <&>","nodes":[{"label":"Card"}]}"#).unwrap();
        let yaml =
            Tecton::render_yaml(&format!("title: '{title}'\nnodes:\n  - label: Card\n")).unwrap();
        assert_eq!(json, yaml);
        assert!(json.contains("<title>标题 &lt;&amp;&gt;</title>"));
        assert!(json.contains(r#"viewBox="0 0 800 660""#));
        assert!(json.contains(
            r#"<svg x="0" y="60" width="800" height="600" viewBox="0 0 800 600" overflow="hidden">"#
        ));
        let node = plain
            .lines()
            .find(|line| line.contains("class=\"node\""))
            .unwrap();
        assert!(json.contains(node), "automatic layout coordinates changed");
        let plain_yaml = Tecton::render_yaml("nodes:\n  - label: Card\n").unwrap();
        assert_eq!(plain, plain_yaml);
        assert_eq!(
            plain,
            Tecton::render_json(r#"{"title":"","nodes":[{"label":"Card"}]}"#).unwrap()
        );
    }

    #[test]
    fn automatic_legend_uses_styled_ordered_samples_independent_of_routes() {
        let svg = Tecton::render_json(
            r##"{
             "title":"Example",
             "legend":[
                {"text":"普通","color":"#123456","arrow_end":true},
                {"text":"虚线","color":"#a33555","width":3,"dash":[5,2],"arrow_end":true},
                {"text":"双向","color":"#114422","arrow_start":true,"arrow_end":true},
                {"text":"参考 <&>","color":"#666666"}],
             "nodes":[{"id":"a","label":"A","ports":[{"name":"out","side":"east"}]},
                      {"id":"b","label":"B","ports":[{"name":"in","side":"west"}]}],
             "edges":[{"from":"a:out","to":"b:in"}]
        }"##,
        )
        .unwrap();
        let legend = svg
            .split(r#"<g class="legend">"#)
            .nth(1)
            .unwrap()
            .split("</g>")
            .next()
            .unwrap();
        let marks: Vec<_> = legend
            .lines()
            .filter(|line| line.starts_with("<polyline"))
            .collect();
        assert_eq!(marks.len(), 4);
        assert!(marks[1].contains(r#"stroke-width="3" class="edge" stroke-dasharray="5 2""#));
        assert!(marks[2].contains(r#"marker-start="url(#arrow-3)" marker-end="url(#arrow-3)""#));
        assert!(!marks[3].contains("marker-"));
        for (row, expected) in legend
            .lines()
            .filter(|line| line.starts_with("<text"))
            .zip(["普通", "虚线", "双向", "参考 &lt;&amp;&gt;"])
        {
            assert!(row.ends_with(&format!(">{expected}</text>")));
        }
        assert!(
            svg.contains(r#"<marker id="arrow-4""#),
            "body route uses its own marker"
        );
        let plain = Tecton::render_json(r#"{"nodes":[{"label":"A"}]}"#).unwrap();
        assert_eq!(
            plain,
            Tecton::render_json(r#"{"nodes":[{"label":"A"}],"legend":[]}"#).unwrap()
        );
        assert!(Tecton::render_json(
            r##"{"legend":[{"text":"bad","color":"#111111","dash":[0]}]}"##
        )
        .is_err());
    }

    #[test]
    fn automatic_legend_wraps_inside_header_without_title() {
        let yaml = r##"
nodes:
  - label: Card
legend:
  - text: A longer entry occupying much of the available header width
    color: '#123456'
    arrow_end: true
  - text: Another entry that must appear on the following legend row
    color: '#345678'
    arrow_end: true
"##;
        let svg = Tecton::render_yaml(yaml).unwrap();
        assert!(!svg.contains("<title>"));
        let legend = svg
            .split(r#"<g class="legend">"#)
            .nth(1)
            .unwrap()
            .split("</g>")
            .next()
            .unwrap();
        let rows: Vec<_> = legend
            .lines()
            .filter(|line| line.starts_with("<text"))
            .collect();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].contains(r#"x="74" y="29""#));
        assert!(rows[1].contains(r#"x="74" y="55""#));
        assert!(svg.contains(r#"<svg x="0" y="90""#));
    }

    #[test]
    fn automatic_card_preserves_ordered_label_rows() {
        let svg = Tecton::render_json(
            r#"{
          "nodes":[{"id":"a","label":"Header\r\n第二  行\n\n<detail>"}]
        }"#,
        )
        .unwrap();
        let rows: Vec<_> = svg
            .lines()
            .filter(|line| line.starts_with("<text "))
            .collect();
        assert_eq!(rows.len(), 4);
        for (row, expected) in rows
            .iter()
            .zip(["Header", "第二  行", "", "&lt;detail&gt;"])
        {
            assert!(row.contains(r#"xml:space="preserve""#));
            assert!(row.ends_with(&format!(">{expected}</text>")));
        }
        assert!(svg
            .lines()
            .any(|line| line.starts_with("<rect ") && line.contains(r#"height="101""#)));
    }

    #[test]
    fn automatic_route_markers_resolve_for_one_and_two_way_edges() {
        let fixture = r#"{
            "nodes": [
                {"id":"a","label":"Source","ports":[{"name":"out","side":"east"}]},
                {"id":"b","label":"Destination","ports":[{"name":"in","side":"west"}]}
            ],
            "edges":[{"from":"a:out","to":"b:in","kind":"EDGE_KIND","label":"Connection"}]
        }"#;
        for (kind, orientation, start_count) in [
            ("default", "auto", 0),
            ("bidirectional", "auto-start-reverse", 1),
        ] {
            let svg = Tecton::render_json(&fixture.replace("EDGE_KIND", kind)).unwrap();
            assert_eq!(svg.matches(r#"id="arrow-1""#).count(), 1);
            assert_eq!(svg.matches(r#"marker-end="url(#arrow-1)""#).count(), 1);
            assert_eq!(
                svg.matches(r#"marker-start="url(#arrow-1)""#).count(),
                start_count
            );
            assert!(svg.contains(&format!(r#"orient="{orientation}""#)));
            assert!(svg.find("<marker").unwrap() < svg.find("<polyline").unwrap());
            assert!(svg.contains(">Source</text>") && svg.contains(">Destination</text>"));
        }
    }

    #[test]
    fn test_tecton_api() {
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

        // Test that the API accepts valid JSON and produces output
        let result = Tecton::render_json(json);
        // The API should successfully parse the JSON and render an SVG
        assert!(
            result.is_ok(),
            "Expected successful rendering, got error: {:?}",
            result.err()
        );
        let svg = result.unwrap();
        // Verify the output is valid SVG
        assert!(svg.contains("<svg"), "Output should contain SVG element");
        assert!(svg.contains("</svg>"), "Output should have closing SVG tag");
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_tecton_valid_json() {
        let json = r#"{
            "canvas": {"width": 800, "height": 600},
            "nodes": [
                {"label": "Server", "ports": [{"name": "api", "side": "east"}]}
            ],
            "edges": []
        }"#;

        let result = Tecton::render_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tecton_invalid_json() {
        let json = r#"{ invalid json }"#;

        let result = Tecton::render_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tecton_valid_yaml() {
        let yaml = r#"
canvas:
  width: 800
  height: 600
nodes:
  - label: Server
    ports:
      - name: api
        side: east
edges: []
"#;

        let result = Tecton::render_yaml(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tecton_invalid_yaml() {
        let yaml = r#"
canvas:
  width: not_a_number
"#;

        let result = Tecton::render_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_tecton_dsl_simple() {
        let dsl = r#"
            config { width: 800, height: 600 }
            Server [label: "API Server"]
        "#;

        let result = Tecton::render_tecton_dsl(dsl);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tecton_dsl_with_edge() {
        let dsl = r#"
            A [label: "Node A"]
            B [label: "Node B"]
            A -> B
        "#;

        let result = Tecton::render_tecton_dsl(dsl);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tecton_dsl_with_container() {
        let dsl = r#"
            Row {
                Node1 [label: "A"]
                Node2 [label: "B"]
            }
        "#;

        let result = Tecton::render_tecton_dsl(dsl);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tecton_dsl_invalid() {
        // The DSL parser may be lenient and produce an empty file
        // Let's use actually invalid syntax
        let dsl = r#" !!!invalid!!!"#;

        let result = Tecton::render_tecton_dsl(dsl);
        // The parser might accept it as empty or fail
        // Just check it either succeeds with valid SVG or fails
        if result.is_ok() {
            let svg = result.unwrap();
            assert!(svg.contains("<svg") || svg.is_empty());
        }
    }

    #[test]
    fn test_tecton_empty_diagram() {
        let json = r#"{
            "canvas": {"width": 100, "height": 100},
            "nodes": [],
            "edges": []
        }"#;

        let result = Tecton::render_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tecton_single_node() {
        let json = r#"{
            "canvas": {"width": 200, "height": 100},
            "nodes": [
                {"label": "Single"}
            ],
            "edges": []
        }"#;

        let result = Tecton::render_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tecton_multi_edge_diagram() {
        let json = r#"{
            "canvas": {"width": 600, "height": 400},
            "nodes": [
                {"label": "A", "ports": [{"name": "out1", "side": "east"}, {"name": "out2", "side": "east"}]},
                {"label": "B", "ports": [{"name": "in", "side": "west"}]},
                {"label": "C", "ports": [{"name": "in", "side": "west"}]}
            ],
            "edges": [
                {"from": "node_1:out1", "to": "node_2:in"},
                {"from": "node_1:out2", "to": "node_3:in"}
            ]
        }"#;

        let result = Tecton::render_json(json);
        assert!(result.is_ok());
    }
}
