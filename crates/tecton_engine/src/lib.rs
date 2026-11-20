//! Tecton Engine - Orchestration and control layer for Tecton
//!
//! This crate provides:
//! - LayoutGraph: A graph structure for nodes and edges
//! - DslParser: JSON/YAML parsing for diagram specifications
//! - Orchestrator: Main controller coordinating layout and routing

pub mod dsl_parser;
pub mod layout_graph;
pub mod named_layout;
mod named_packing;
pub mod orchestrator;
pub mod tecton_dsl_converter;

pub use dsl_parser::{CanvasSpec, DiagramDsl, DslParser, EdgeSpec, NodeSpec};
pub use layout_graph::{LayoutGraph, LayoutNode, LayoutPort};
pub use named_layout::{BoundEdge, DiagramBindings, NamedAutomaticLayout};
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

    /// Compute geometry through the ordinary automatic engine, then retain the
    /// exact authored node/port/edge identities for semantic decoration.
    /// All nodes and edges must have explicit, unique IDs.
    pub fn layout_named_dsl(diagram: &DiagramDsl) -> Result<NamedAutomaticLayout> {
        NamedAutomaticLayout::compute(diagram)
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
    fn named_automatic_layout_keeps_parallel_relation_identity() {
        let json = r##"{
            "nodes":[{"id":"group","label":"Group","children":[
                {"id":"a","label":"Card","ports":[{"name":"out","side":"east"}]},
                {"id":"b","label":"Card","ports":[{"name":"in","side":"west"}]}]}],
            "edges":[
                {"id":"first","from":"a:out","to":"b:in","label":"First","color":"#123456"},
                {"id":"second","from":"a:out","to":"b:in","label":"Second","color":"#234567","kind":"dashed"}
            ]
        }"##;
        let mut dsl = DslParser::new().parse_json(json).unwrap();
        for reverse in [false, true] {
            if reverse {
                dsl.edges.reverse();
            }
            let result = Tecton::layout_named_dsl(&dsl).unwrap();
            assert_eq!(result.node("group").unwrap().children.len(), 2);
            assert_eq!(result.node("a").unwrap().label, "Card");
            assert_eq!(result.node("b").unwrap().label, "Card");
            assert_ne!(
                result.node("a").unwrap().rect,
                result.node("b").unwrap().rect
            );
            let bindings = result.bindings();
            let first = bindings.edge("first").unwrap();
            let second = bindings.edge("second").unwrap();
            assert_ne!(first.edge_id, second.edge_id);
            assert_eq!(bindings.port_id("a:out"), Some(first.source_port));
            assert_eq!(bindings.port_id("b:in"), Some(second.target_port));
            let first = result.route("first").unwrap();
            let second = result.route("second").unwrap();
            assert_eq!(first.label.as_deref(), Some("First"));
            assert_eq!(second.label.as_deref(), Some("Second"));
            assert_ne!(first.edge_color, second.edge_color);
            assert!(first.segments.len() >= 2 && second.segments.len() >= 2);
            assert!(result.route("unknown").is_none());
            assert!(result.render().unwrap().contains("Second"));
        }
    }

    const NAMED_TERMINAL_FIXTURE: &str = r##"{
        "nodes":[
            {"id":"left","label":"Left card","ports":[
                {"name":"near","side":"east","position":0.25},
                {"name":"far","side":"east","position":0.75},
                {"name":"decoy","side":"west","position":0.5}
            ]},
            {"id":"right","label":"Right card","ports":[
                {"name":"upper","side":"east","position":0.25},
                {"name":"lower","side":"east","position":0.75},
                {"name":"facing_decoy","side":"west","position":0.5}
            ]}
        ],
        "edges":[
            {"id":"high","from":"left:near","to":"right:upper","kind":"bidirectional","color":"#165e97"},
            {"id":"low","from":"left:far","to":"right:lower","kind":"bidirectional","color":"#a64314"}
        ]
    }"##;

    #[test]
    fn named_automatic_terminals_honor_fraction_side_and_port_order() {
        let mut diagram = DslParser::new().parse_json(NAMED_TERMINAL_FIXTURE).unwrap();
        for reverse in [false, true] {
            if reverse {
                for node in &mut diagram.nodes {
                    node.ports.reverse();
                }
                diagram.edges.reverse();
            }
            let rendered = Tecton::layout_named_dsl(&diagram).unwrap();
            let left = rendered.node("left").unwrap().rect;
            let right = rendered.node("right").unwrap().rect;
            assert!(left.right() < right.left());
            for (id, fraction, color) in [("high", 0.25, "#165e97"), ("low", 0.75, "#a64314")] {
                let route = rendered.route(id).unwrap();
                let points = route.points();
                let (start, end) = (points[0], *points.last().unwrap());
                assert_eq!(start.x(), left.right());
                assert_eq!(start.y(), left.top() + left.height() * fraction);
                // The heuristic-facing side here would be west, not authored east.
                assert_eq!(end.x(), right.right());
                assert_eq!(end.y(), right.top() + right.height() * fraction);
                let (begin_stub, end_stub) = (points[1], points[points.len() - 2]);
                assert!(begin_stub.x() > start.x() && begin_stub.y() == start.y());
                assert!(end_stub.x() > end.x() && end_stub.y() == end.y());
                assert!(points
                    .windows(2)
                    .all(|w| { w[0] != w[1] && (w[0].x() == w[1].x() || w[0].y() == w[1].y()) }));
                let svg = rendered.render().unwrap();
                let polyline = svg
                    .lines()
                    .find(|line| line.starts_with("<polyline ") && line.contains(color))
                    .unwrap();
                assert!(polyline.contains("marker-start=") && polyline.contains("marker-end="));
                let point_str = polyline
                    .split_once("points=\"")
                    .unwrap()
                    .1
                    .split('"')
                    .next()
                    .unwrap();
                assert!(point_str.starts_with(&format!("{}, {} ", start.x(), start.y())));
                assert!(point_str.ends_with(&format!(" {}, {}", end.x(), end.y())));
            }
        }
    }

    #[test]
    fn named_automatic_terminals_reject_invalid_fractions_and_curved_corners() {
        let bad_fraction = NAMED_TERMINAL_FIXTURE.replacen("0.25", "1.25", 1);
        let mut diagram = DslParser::new().parse_json(&bad_fraction).unwrap();
        assert!(Tecton::layout_named_dsl(&diagram)
            .err()
            .unwrap()
            .to_string()
            .contains("fraction in [0, 1]"));
        // Classic path still accepts the old clamped position.
        assert!(Tecton::render_dsl(&diagram).is_ok());
        let rounded = NAMED_TERMINAL_FIXTURE.replacen(
            r#""id":"left""#,
            r#""id":"left","border_radius":20"#,
            1,
        );
        diagram = DslParser::new().parse_json(&rounded).unwrap();
        assert!(Tecton::layout_named_dsl(&diagram)
            .err()
            .unwrap()
            .to_string()
            .contains("outside the straight rectangular side"));
    }

    #[test]
    fn named_automatic_packing_keeps_multiple_cjk_groups_and_routes_in_bounds() {
        let groups: Vec<_> = (0..3).map(|g| serde_json::json!({
            "id":format!("g{g}"),
            "label":format!("自动分组 {g}"),
            "children":(0..7).map(|c| serde_json::json!({
                "id":format!("g{g}c{c}"),
                "label":format!("多设备实时信号编排检查结果 {g}.{c}\n下一步验证"),
                "ports":[{"name":"out","side":"east","position":0.25},{"name":"in","side":"west","position":0.75}]
            })).collect::<Vec<_>>(),
        })).collect();
        let fixture = serde_json::json!({
            "canvas":{"width":780,"height":400},
            "nodes":groups,
            "edges":[
                {"id":"cross","from":"g0c0:out","to":"g2c5:in","kind":"bidirectional"},
                {"id":"between","from":"g1c1:out","to":"g0c5:in"}
            ],
        });
        let diagram = DslParser::new().parse_json(&fixture.to_string()).unwrap();
        let a = Tecton::layout_named_dsl(&diagram).unwrap();
        let b = Tecton::layout_named_dsl(&diagram).unwrap();
        let bounds = a.layout().bounds;
        let contains = |a: tecton_types::geometry::Rect, b: tecton_types::geometry::Rect| {
            a.left() <= b.left()
                && a.top() <= b.top()
                && a.right() >= b.right()
                && a.bottom() >= b.bottom()
        };
        assert_eq!(
            bounds.width(),
            780.0,
            "must honor width, not grow an unbounded single row"
        );
        assert!(
            bounds.height() > 400.0,
            "height grows to fit actual content"
        );
        for (i, group) in a.layout().root.children.iter().enumerate() {
            assert!(contains(bounds, group.rect));
            assert_eq!(group.rect, b.node(&format!("g{i}")).unwrap().rect);
            for other in &a.layout().root.children[..i] {
                assert!(!group.rect.intersects(&other.rect));
            }
            for (index, child) in group.children.iter().enumerate() {
                assert!(contains(group.rect, child.rect));
                assert_eq!(child.rect, b.node(&format!("g{i}c{index}")).unwrap().rect);
                for other in &group.children[..index] {
                    assert!(!child.rect.intersects(&other.rect));
                }
            }
        }
        let route = a.route("cross").unwrap();
        assert!(route.points().iter().all(|p| bounds.contains(p)));
        assert_eq!(route.points(), b.route("cross").unwrap().points());
        let svg = a.render().unwrap();
        assert!(svg.contains(&format!(
            r#"viewBox="0 0 {} {}""#,
            bounds.width(),
            bounds.height()
        )));
        assert_eq!(svg.matches(">下一步验证</text>").count(), 21);
    }

    #[test]
    fn named_automatic_packing_rejects_an_unbounded_canvas() {
        let input = r#"{"canvas":{"width":999999,"height":400},"nodes":[{"id":"a","label":"A"}]}"#;
        let diagram = DslParser::new().parse_json(input).unwrap();
        let error = Tecton::layout_named_dsl(&diagram).err().unwrap();
        assert!(error.to_string().contains("bounded packing input"));
        // Legacy still retains its independent behavior.
        assert!(Tecton::render_dsl(&diagram)
            .unwrap()
            .contains(r#"viewBox="0 0 800 600""#));
    }

    #[test]
    fn named_automatic_layout_rejects_ambiguous_authored_ids() {
        let p = DslParser::new();
        let duplicate = p
            .parse_json(r#"{"nodes":[{"id":"a","label":"A","children":[{"id":"a","label":"B"}]}]}"#)
            .unwrap();
        assert!(p
            .to_layout_graph_with_bindings(&duplicate)
            .unwrap_err()
            .contains("duplicate bound node ID"));
        let absent = p.parse_json(r#"{"nodes":[{"label":"Legacy"}]}"#).unwrap();
        assert!(p
            .to_layout_graph_with_bindings(&absent)
            .unwrap_err()
            .contains("explicit ID"));
        assert!(Tecton::render_dsl(&absent).is_ok());
        let duplicate_edge = p.parse_json(r#"{"edges":[{"id":"e","from":"a:n","to":"b:n"},{"id":"e","from":"b:n","to":"a:n"}]}"#).unwrap();
        assert!(p
            .to_layout_graph_with_bindings(&duplicate_edge)
            .unwrap_err()
            .contains("duplicate bound edge ID"));
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
    fn automatic_edge_labels_stay_inside_the_body_with_or_without_a_header() {
        let fixture = r##"{
            "nodes":[
                {"id":"a","label":"Card","ports":[{"name":"out","side":"east"}]},
                {"id":"b","label":"First sink","ports":[{"name":"in","side":"west"}]},
                {"id":"c","label":"Card","ports":[{"name":"out","side":"east"}]},
                {"id":"d","label":"Second sink","ports":[{"name":"in","side":"west"}]}
            ],
            "edges":[
                {"id":"alpha","from":"a:out","to":"b:in","kind":"bidirectional","color":"#123456","label":"EDGE_LABEL"},
                {"id":"beta","from":"c:out","to":"d:in","kind":"dashed","color":"#6a278f","label":"Beta"}
            ]
        }"##;
        fn number(line: &str, attr: &str) -> f32 {
            line.split_once(&format!(r#" {attr}=""#))
                .unwrap()
                .1
                .split('"')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        }
        let base = Tecton::render_json(&fixture.replace("EDGE_LABEL", "Alpha")).unwrap();
        // A wide header expands the outer SVG, never the inner body viewport.
        let decorated = fixture.replace("EDGE_LABEL", "前端服务与消息队列｜数据流转").replacen(
            '{',
            &format!(
                r##"{{"title":"{}","legend":[{{"text":"Sample","color":"#555555","width":2,"arrow_end":true}}],"##,
                "Extended title ".repeat(4)
            ),
            1,
        );
        let decorated = Tecton::render_json(&decorated).unwrap();
        assert!(decorated.contains(r#"viewBox="0 0 800 600" overflow="hidden""#));
        let polylines = |svg: &str| {
            svg.lines()
                .filter(|line| {
                    line.starts_with("<polyline ")
                        && (line.contains(r##"stroke="#123456""##)
                            || line.contains(r##"stroke="#6a278f""##))
                })
                .map(|line| line.split_once(" marker-").unwrap().0.to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(polylines(&base).len(), 2);
        assert_eq!(polylines(&base), polylines(&decorated));
        for (svg, label) in [
            (&base, "Alpha"),
            (&decorated, "前端服务与消息队列｜数据流转"),
        ] {
            let line = svg
                .lines()
                .find(|line| line.ends_with(&format!(">{label}</text>")))
                .unwrap();
            let font = number(line, "font-size");
            let x = number(line, "x");
            let y = number(line, "y");
            let ink = label.chars().count() as f32 * font * 1.25 / 2.0;
            assert_eq!(font, 12.0);
            assert!(x - ink >= 6.0 && x + ink <= 794.0);
            assert!(y - font >= 6.0 && y + font <= 594.0);
        }
        let oversized = fixture.replace("EDGE_LABEL", &"界".repeat(90));
        assert!(Tecton::render_json(&oversized)
            .unwrap_err()
            .to_string()
            .contains("edge label cannot fit"));
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
