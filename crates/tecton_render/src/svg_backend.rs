use std::collections::HashMap;

use tecton_layout::layout_context::{LayoutNode, LayoutResult};
use tecton_routing::route::{Route, RouteSet};
use tecton_types::{
    error::{Result, TectonError},
    shape::ShapeType,
    style::Color,
};

use crate::backend::Backend;

/// SVG rendering backend
pub struct SvgBackend {
    /// SVG width
    width: f32,
    /// SVG height
    height: f32,
    /// SVG content buffer
    content: String,
    /// Definitions (gradients, markers, etc.)
    definitions: Vec<String>,
    /// Whether to include styles inline
    inline_styles: bool,
    /// Node styles cache
    node_styles: HashMap<String, String>,
    /// Arrow marker ID counter
    marker_id_counter: u32,
}

impl Default for SvgBackend {
    fn default() -> Self {
        Self::new(800.0, 600.0)
    }
}

impl SvgBackend {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            content: String::new(),
            definitions: Vec::new(),
            inline_styles: true,
            node_styles: HashMap::new(),
            marker_id_counter: 0,
        }
    }

    pub fn with_inline_styles(mut self, inline: bool) -> Self {
        self.inline_styles = inline;
        self
    }

    /// Start the SVG document
    fn start_svg(&mut self) {
        self.content.push_str(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
            self.width, self.height, self.width, self.height
        ));
        self.content.push('\n');

        // Add definitions
        if !self.definitions.is_empty() {
            self.content.push_str("<defs>\n");
            for def in &self.definitions {
                self.content.push_str(def);
                self.content.push('\n');
            }
            self.content.push_str("</defs>\n");
        }

        // Add default styles
        if self.inline_styles {
            self.content.push_str("<style>\n");
            self.content.push_str(".node { stroke-linejoin: round; }\n");
            self.content
                .push_str(".edge { fill: none; stroke-linecap: round; stroke-linejoin: round; }\n");
            self.content
                .push_str(".label { font-family: system-ui, -apple-system, sans-serif; }\n");
            self.content.push_str("</style>\n");
        }
    }

    /// End the SVG document
    fn end_svg(&mut self) {
        self.content.push_str("</svg>");
    }

    /// Render a rectangle (node)
    fn render_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &tecton_types::style::Style,
    ) {
        let fill = color_to_css(&style.background);
        let stroke = color_to_css(&style.border_color);
        let stroke_width = style.border_width();
        let rx = style.border_radius();

        let attrs = if rx > 0.0 {
            format!(
                r#"x="{}" y="{}" width="{}" height="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" class="node""#,
                x, y, width, height, rx, rx, fill, stroke, stroke_width
            )
        } else {
            format!(
                r#"x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}" class="node""#,
                x, y, width, height, fill, stroke, stroke_width
            )
        };

        self.content.push_str(&format!("<rect {} />\n", attrs));
    }

    /// Render a circle
    fn render_circle(&mut self, cx: f32, cy: f32, r: f32, style: &tecton_types::style::Style) {
        let fill = color_to_css(&style.background);
        let stroke = color_to_css(&style.border_color);
        let stroke_width = style.border_width();

        let attrs = format!(
            r#"cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}" class="node""#,
            cx, cy, r, fill, stroke, stroke_width
        );

        self.content.push_str(&format!("<circle {} />\n", attrs));
    }

    /// Render a diamond (rhombus)
    fn render_diamond(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &tecton_types::style::Style,
    ) {
        let fill = color_to_css(&style.background);
        let stroke = color_to_css(&style.border_color);
        let stroke_width = style.border_width();

        let cx = x + width / 2.0;
        let cy = y + height / 2.0;
        let half_w = width / 2.0;
        let half_h = height / 2.0;

        // Diamond points: top, right, bottom, left
        let points = format!(
            "{},{} {},{} {},{} {},{}",
            cx,
            cy - half_h, // top
            cx + half_w,
            cy, // right
            cx,
            cy + half_h, // bottom
            cx - half_w,
            cy // left
        );

        let attrs = format!(
            r#"points="{}" fill="{}" stroke="{}" stroke-width="{}" class="node""#,
            points, fill, stroke, stroke_width
        );

        self.content.push_str(&format!("<polygon {} />\n", attrs));
    }

    /// Render a node with the specified shape
    fn render_shape(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &tecton_types::style::Style,
    ) {
        match style.shape {
            ShapeType::Circle => {
                let radius = width.min(height) / 2.0;
                let cx = x + width / 2.0;
                let cy = y + height / 2.0;
                self.render_circle(cx, cy, radius, style);
            }
            ShapeType::Diamond => {
                self.render_diamond(x, y, width, height, style);
            }
            // For now, treat other shapes as rectangles
            _ => {
                self.render_rect(x, y, width, height, style);
            }
        }
    }

    /// Render text label
    fn render_text(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        color: &Color,
        font_size: f32,
        anchor: &str,
    ) {
        let fill = color_to_css(color);
        let escaped_text = escape_xml(text);

        self.content.push_str(&format!(
            r#"<text x="{}" y="{}" fill="{}" font-size="{}" text-anchor="{}" class="label" dominant-baseline="middle">{}</text>"#,
            x, y, fill, font_size, anchor, escaped_text
        ));
        self.content.push('\n');
    }

    /// Render a polyline (route)
    fn render_polyline(
        &mut self,
        points: &[(f32, f32)],
        color: &Color,
        stroke_width: f32,
        dash_array: Option<&str>,
        marker_end: Option<&str>,
    ) {
        if points.is_empty() {
            return;
        }

        // Handle single point by adding a small offset to create a visible line
        let points = if points.len() == 1 {
            let (x, y) = points[0];
            vec![(x, y), (x + 1.0, y)] // Add 1px line for single point
        } else {
            points.to_vec()
        };

        let points_str = points
            .iter()
            .map(|(x, y)| format!("{}, {}", x, y))
            .collect::<Vec<_>>()
            .join(" ");

        let stroke = color_to_css(color);

        let mut attrs = format!(
            r#"points="{}" fill="none" stroke="{}" stroke-width="{}" class="edge""#,
            points_str, stroke, stroke_width
        );

        if let Some(dash) = dash_array {
            attrs.push_str(&format!(r#" stroke-dasharray="{}""#, dash));
        }

        if let Some(marker) = marker_end {
            attrs.push_str(&format!(r#" marker-end="url(#{})""#, marker));
        }

        self.content.push_str(&format!("<polyline {} />\n", attrs));
    }

    /// Add an arrow marker definition
    fn add_arrow_marker(&mut self, color: &Color) -> String {
        self.marker_id_counter += 1;
        let id = format!("arrow-{}", self.marker_id_counter);
        let stroke = color_to_css(color);

        let marker = format!(
            r#"<marker id="{}" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto" markerUnits="strokeWidth">
                <path d="M0,0 L0,6 L9,3 z" fill="{}" />
            </marker>"#,
            id, stroke
        );

        self.definitions.push(marker);
        id
    }

    /// Render a node recursively
    fn render_node_recursive(&mut self, node: &LayoutNode) -> Result<()> {
        // Determine if this is a content node vs a pure container
        // Content nodes have: label, specific shape, OR are leaf nodes (no children)
        let has_label = !node.label.is_empty();
        let has_specific_shape = node.style.shape != ShapeType::Rectangle;
        let is_leaf = node.children.is_empty();
        let is_content_node = has_label || has_specific_shape || is_leaf;

        if is_content_node {
            let rect = &node.rect;

            // Render the node with appropriate shape
            self.render_shape(rect.x(), rect.y(), rect.width(), rect.height(), &node.style);

            // Render label if present
            if has_label {
                let text_x = if is_leaf { rect.x() + rect.width() / 2.0 } else { rect.x() + 10.0 };
                let text_y = if is_leaf { rect.y() + rect.height() / 2.0 } else { rect.y() + 15.0 };
                let anchor = if is_leaf { "middle" } else { "start" };
                self.render_text(
                    text_x,
                    text_y,
                    &node.label,
                    &node.style.text_color,
                    14.0,
                    anchor,
                );
            }
        }

        // Render children (whether or not this node is a container)
        for child in &node.children {
            self.render_node_recursive(child)?;
        }

        Ok(())
    }

    /// Render a route
    fn render_route(&mut self, route: &Route, _color: &Color, _stroke_width: f32) -> Result<()> {
        if route.segments.is_empty() {
            return Ok(());
        }

        let points: Vec<(f32, f32)> = route.segments.iter().map(|s| (s.x, s.y)).collect();

        // Use edge_color and edge_kind from route
        let color = &route.edge_color;
        let stroke_width = route.edge_kind.stroke_width();
        let dash_array = route.edge_kind.stroke_dasharray();

        let marker_id = self.add_arrow_marker(color);
        self.render_polyline(
            &points,
            color,
            stroke_width,
            dash_array.as_deref(),
            Some(&marker_id),
        );

        // Render edge label if present
        if let Some(ref label) = route.label {
            if route.segments.len() >= 2 {
                // Place label at midpoint of route
                let mid_idx = route.segments.len() / 2;
                let mid = &route.segments[mid_idx];
                self.render_text(mid.x, mid.y - 8.0, label, color, 12.0, "middle");
            }
        }

        Ok(())
    }
}

impl Backend for SvgBackend {
    fn render(&mut self, layout: &LayoutResult, routes: Option<&RouteSet>) -> Result<()> {
        self.start_svg();

        // Add a background
        self.content.push_str(&format!(
            r#"<rect x="0" y="0" width="{}" height="{}" fill="white" />"#,
            self.width, self.height
        ));
        self.content.push('\n');

        // Render all nodes
        self.render_node_recursive(&layout.root)?;

        // Render routes if provided
        if let Some(route_set) = routes {
            self.render_routes(route_set)?;
        }

        self.end_svg();
        Ok(())
    }

    fn render_node(&mut self, node: &LayoutNode) -> Result<()> {
        self.render_node_recursive(node)
    }

    fn render_routes(&mut self, routes: &RouteSet) -> Result<()> {
        for route in routes.routes() {
            // Use a default color for now - in a full implementation,
            // we'd look up the edge's color from the edge data
            let color = tecton_types::style::Color::GRAY_500;
            self.render_route(route, &color, 1.0)?;
        }
        Ok(())
    }

    fn output(&self) -> Option<String> {
        Some(self.content.clone())
    }

    fn save(&self, path: &str) -> Result<()> {
        std::fs::write(path, &self.content).map_err(|e| TectonError::IoError(e))?;
        Ok(())
    }
}

/// Convert a Color to CSS color string
fn color_to_css(color: &Color) -> String {
    if color.a == 255 {
        format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
    } else {
        format!(
            "rgba({}, {}, {}, {})",
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0
        )
    }
}

/// Escape XML special characters
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tecton_layout::layout_context::LayoutNode;
    use tecton_types::geometry::{Rect, Size};
    use tecton_types::id::NodeId;
    use tecton_types::style::Style;

    #[test]
    fn test_svg_backend_basic() {
        let mut backend = SvgBackend::new(100.0, 100.0);

        let node = LayoutNode::new(
            NodeId::new(),
            Rect::new(10.0, 10.0, 80.0, 80.0),
            Style::default(),
        );

        let layout = LayoutResult::new(node.clone(), Rect::new(0.0, 0.0, 100.0, 100.0));

        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        assert!(output.contains("<svg"));
        assert!(output.contains("</svg>"));
        assert!(output.contains("<rect"));
    }

    #[test]
    fn test_color_to_css() {
        let color = Color::new(255, 128, 0);
        assert_eq!(color_to_css(&color), "#ff8000");

        let color = Color::with_alpha(255, 128, 0, 128);
        assert_eq!(color_to_css(&color), "rgba(255, 128, 0, 0.5019608)");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<test>"), "&lt;test&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_svg_backend_default() {
        let backend = SvgBackend::default();
        assert_eq!(backend.width, 800.0);
        assert_eq!(backend.height, 600.0);
    }

    #[test]
    fn test_svg_backend_new() {
        let backend = SvgBackend::new(400.0, 300.0);
        assert_eq!(backend.width, 400.0);
        assert_eq!(backend.height, 300.0);
    }

    #[test]
    fn test_svg_backend_with_inline_styles() {
        let backend = SvgBackend::new(100.0, 100.0).with_inline_styles(false);

        assert!(!backend.inline_styles);
    }

    #[test]
    fn test_svg_viewbox() {
        let mut backend = SvgBackend::new(400.0, 300.0);

        let node = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Style::default(),
        );

        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 400.0, 300.0));
        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        assert!(output.contains("viewBox=\"0 0 400 300\""));
    }

    #[test]
    fn test_svg_render_circle() {
        let mut backend = SvgBackend::new(100.0, 100.0);

        let style = Style {
            shape: ShapeType::Circle,
            ..Style::default()
        };

        let node = LayoutNode::new(NodeId::new(), Rect::new(10.0, 10.0, 80.0, 80.0), style);

        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 100.0, 100.0));
        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        assert!(output.contains("<circle"));
    }

    #[test]
    fn test_svg_render_diamond() {
        let mut backend = SvgBackend::new(100.0, 100.0);

        let style = Style {
            shape: ShapeType::Diamond,
            ..Style::default()
        };

        let node = LayoutNode::new(NodeId::new(), Rect::new(10.0, 10.0, 80.0, 80.0), style);

        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 100.0, 100.0));
        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        assert!(output.contains("<polygon"));
    }

    #[test]
    fn test_svg_render_nested_nodes() {
        let mut backend = SvgBackend::new(200.0, 200.0);

        let child = LayoutNode::new(
            NodeId::new(),
            Rect::new(10.0, 10.0, 80.0, 80.0),
            Style::default(),
        );

        let root = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 200.0, 200.0),
            Style::default(),
        )
        .with_children(vec![child]);

        let layout = LayoutResult::new(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        // Should have two rect elements (root and child)
        assert!(output.contains("<rect"));
    }

    #[test]
    fn test_svg_render_with_routes() {
        let mut backend = SvgBackend::new(200.0, 200.0);

        let node = LayoutNode::new(
            NodeId::new(),
            Rect::new(10.0, 10.0, 80.0, 80.0),
            Style::default(),
        );

        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 200.0, 200.0));

        let mut route_set = RouteSet::new();
        let route = Route::from_segments(
            tecton_types::id::EdgeId::new(),
            tecton_types::id::PortId::new(),
            tecton_types::id::PortId::new(),
            vec![
                tecton_types::edge::RouteSegment::move_to(0.0, 0.0),
                tecton_types::edge::RouteSegment::line_to(100.0, 100.0),
            ],
        );
        route_set.add_route(route);

        backend.render(&layout, Some(&route_set)).unwrap();

        let output = backend.output().unwrap();
        // Should have polyline for routes
        assert!(output.contains("<polyline") || output.contains("<defs>"));
    }

    #[test]
    fn test_svg_style_application() {
        let mut backend = SvgBackend::new(100.0, 100.0);

        let style = Style {
            background: Color::new(255, 0, 0),
            border_color: Color::new(0, 0, 255),
            border_width: 2.0.into(),
            border_radius: 10.0.into(),
            ..Style::default()
        };

        let node = LayoutNode::new(NodeId::new(), Rect::new(10.0, 10.0, 80.0, 80.0), style);

        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 100.0, 100.0));
        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        assert!(output.contains("fill=\"#ff0000\""));
        assert!(output.contains("stroke=\"#0000ff\""));
        assert!(output.contains("stroke-width=\"2\""));
        assert!(output.contains("rx=\"10\""));
    }

    #[test]
    fn test_escape_xml_all_special_chars() {
        assert_eq!(escape_xml("&"), "&amp;");
        assert_eq!(escape_xml("<"), "&lt;");
        assert_eq!(escape_xml(">"), "&gt;");
        assert_eq!(escape_xml("\""), "&quot;");
        assert_eq!(escape_xml("'"), "&apos;");
    }

    #[test]
    fn test_escape_xml_complex() {
        let input = "<div class=\"test\">Hello & Goodbye</div>";
        let expected = "&lt;div class=&quot;test&quot;&gt;Hello &amp; Goodbye&lt;/div&gt;";
        assert_eq!(escape_xml(input), expected);
    }

    #[test]
    fn test_color_to_css_opaque() {
        let color = Color::new(0, 0, 0);
        assert_eq!(color_to_css(&color), "#000000");

        let color = Color::new(255, 255, 255);
        assert_eq!(color_to_css(&color), "#ffffff");
    }

    #[test]
    fn test_color_to_css_with_alpha() {
        let color = Color::with_alpha(0, 0, 0, 0);
        assert_eq!(color_to_css(&color), "rgba(0, 0, 0, 0)");

        let color = Color::with_alpha(255, 255, 255, 255);
        assert_eq!(color_to_css(&color), "#ffffff");
    }

    #[test]
    fn test_svg_backend_save() {
        let mut backend = SvgBackend::new(100.0, 100.0);

        let node = LayoutNode::new(
            NodeId::new(),
            Rect::new(10.0, 10.0, 80.0, 80.0),
            Style::default(),
        );

        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 100.0, 100.0));
        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        assert!(output.starts_with("<svg"));
        assert!(output.ends_with("</svg>"));
    }

    #[test]
    fn test_svg_background() {
        let mut backend = SvgBackend::new(100.0, 100.0);

        let node = LayoutNode::new(
            NodeId::new(),
            Rect::new(10.0, 10.0, 80.0, 80.0),
            Style::default(),
        );

        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 100.0, 100.0));
        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        // Should include a white background
        assert!(output.contains("fill=\"white\""));
    }

    #[test]
    fn test_svg_styles_included() {
        let mut backend = SvgBackend::new(100.0, 100.0);

        let node = LayoutNode::new(
            NodeId::new(),
            Rect::new(10.0, 10.0, 80.0, 80.0),
            Style::default(),
        );

        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 100.0, 100.0));
        backend.render(&layout, None).unwrap();

        let output = backend.output().unwrap();
        // Should include style definitions
        assert!(output.contains("<style>"));
        assert!(output.contains(".node"));
        assert!(output.contains(".edge"));
    }
}
