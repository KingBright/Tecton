use std::collections::HashMap;

use tecton_layout::layout_context::{
    node_label_lines, LayoutNode, LayoutResult, NODE_LABEL_LINE_HEIGHT,
};
use tecton_routing::route::{Route, RouteSet};
use tecton_types::{
    edge::EdgeKind,
    error::{Result, TectonError},
    shape::ShapeType,
    style::Color,
};

use crate::{
    backend::Backend,
    fixed_scene::{dash_css, paint, xml_text_safe, FixedNode, FixedScene, LineKind, TextRun},
    legend::LegendEntry,
};

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

    /// Render an explicitly positioned scene without invoking layout or routing.
    /// Visual routes are drawn exactly once, even if multiple logical edges refer to them.
    pub fn render_fixed_scene(scene: &FixedScene) -> Result<String> {
        scene.validate()?;
        let mut svg = Self::new(scene.canvas[0], scene.canvas[1]);
        let markers = scene
            .edges
            .iter()
            .map(|route| {
                let color = paint(&route.color)?;
                Ok(if route.arrow_start || route.arrow_end {
                    color.map(|color| {
                        svg.add_arrow_marker_with_orientation(&color, "auto-start-reverse")
                    })
                } else {
                    None
                })
            })
            .collect::<Result<Vec<_>>>()?;
        svg.start_svg();
        svg.render_background();
        for node in &scene.nodes {
            svg.render_fixed_rects(node, false)?;
        }
        for (route, marker) in scene.edges.iter().zip(markers.iter()) {
            let points: Vec<_> = route.points.iter().map(|[x, y]| (*x, *y)).collect();
            let dash = route
                .dash
                .as_deref()
                .map(dash_css)
                .or_else(|| matches!(route.kind, LineKind::Dashed).then(|| "6 4".to_owned()));
            svg.render_styled_polyline(
                &points,
                paint_to_css(&route.color)?.as_str(),
                route.width,
                dash.as_deref(),
                marker.as_deref().filter(|_| route.arrow_start),
                marker.as_deref().filter(|_| route.arrow_end),
            );
        }
        for node in &scene.nodes {
            svg.render_fixed_rects(node, true)?;
        }
        for node in &scene.nodes {
            svg.render_fixed_texts(node)?;
        }
        for text in &scene.annotations {
            svg.render_fixed_text(text)?;
        }
        svg.end_svg();
        Ok(svg.content)
    }

    /// Keep header geometry separate from the untouched automatic layout viewport.
    pub fn render_automatic_with_title(
        width: f32,
        height: f32,
        layout: &LayoutResult,
        routes: Option<&RouteSet>,
        title: &str,
    ) -> Result<String> {
        Self::render_automatic_with_header(width, height, layout, routes, title, &[])
    }

    /// Render optional, geometry-free document metadata above the graph viewport.
    pub fn render_automatic_with_header(
        width: f32,
        height: f32,
        layout: &LayoutResult,
        routes: Option<&RouteSet>,
        title: &str,
        legend: &[LegendEntry],
    ) -> Result<String> {
        if title.is_empty() && legend.is_empty() {
            let mut svg = Self::new(width, height);
            svg.render(layout, routes)?;
            return Ok(svg.content);
        }
        if !xml_text_safe(title) {
            return Err(TectonError::invalid_input("invalid SVG document title"));
        }

        const FONT: f32 = 22.0;
        const ROW_HEIGHT: f32 = 28.0;
        const PADDING: f32 = 16.0;
        for entry in legend {
            entry.validate()?;
        }
        let (widest, title_count) = if title.is_empty() {
            (0, 0)
        } else {
            node_label_lines(title).fold((0, 0), |(widest, count), row| {
                (widest.max(row.chars().count()), count + 1)
            })
        };
        let legend_item_width = legend
            .iter()
            .map(LegendEntry::item_width)
            .fold(0.0, f32::max);
        let header_width = (2.0 * PADDING + widest as f32 * FONT * 1.25)
            .max(2.0 * PADDING + legend_item_width)
            .max(width);
        let row_height = legend
            .iter()
            .map(|item| item.width * 6.0 + 12.0)
            .fold(26.0, f32::max);
        let gap_after_title = if title_count > 0 && !legend.is_empty() {
            6.0
        } else {
            0.0
        };
        let legend_top = PADDING + title_count as f32 * ROW_HEIGHT + gap_after_title;
        let mut x = PADDING;
        let mut row = 0;
        let mut positions = Vec::with_capacity(legend.len());
        for entry in legend {
            let item_width = entry.item_width();
            if x > PADDING && x + item_width > header_width - PADDING {
                x = PADDING;
                row += 1;
            }
            positions.push((x, legend_top + (row as f32 + 0.5) * row_height));
            x += item_width + 22.0;
        }
        let legend_height = if legend.is_empty() {
            0.0
        } else {
            (row + 1) as f32 * row_height
        };
        let header_height = legend_top + legend_height + PADDING;
        // A stable header height on the default 800→720 preview pixel grid.
        let header_height = if legend.is_empty() {
            header_height
        } else {
            (header_height / 10.0).ceil() * 10.0
        };
        let mut svg = Self::new(header_width, height + header_height);
        let mut markers = Vec::with_capacity(legend.len());
        for entry in legend {
            markers.push(if entry.arrow_start || entry.arrow_end {
                paint(&entry.color)?.map(|color| {
                    svg.add_arrow_marker_with_orientation(&color, "auto-start-reverse")
                })
            } else {
                None
            });
        }
        svg.start_svg();
        svg.render_background();
        if !title.is_empty() {
            svg.content
                .push_str(&format!("<title>{}</title>\n", escape_xml(title)));
            for (index, row) in node_label_lines(title).enumerate() {
                svg.render_text_row(
                    PADDING,
                    PADDING + (index as f32 + 0.5) * ROW_HEIGHT,
                    row,
                    &Color::BLACK,
                    FONT,
                    "start",
                    true,
                );
            }
        }
        if !legend.is_empty() {
            svg.content.push_str("<g class=\"legend\">\n");
            for ((entry, &(x, y)), marker) in legend.iter().zip(&positions).zip(&markers) {
                let end = x + entry.sample_length();
                let dash = entry.dash.as_deref().map(dash_css);
                svg.render_styled_polyline(
                    &[(x, y), (end, y)],
                    &paint_to_css(&entry.color)?,
                    entry.width,
                    dash.as_deref(),
                    marker.as_deref().filter(|_| entry.arrow_start),
                    marker.as_deref().filter(|_| entry.arrow_end),
                );
                svg.render_text_row(
                    end + 10.0,
                    y,
                    &entry.text,
                    &Color::BLACK,
                    LegendEntry::FONT_SIZE,
                    "start",
                    true,
                );
            }
            svg.content.push_str("</g>\n");
        }
        // Explicitly clip the old viewport exactly as it was before adding the header.
        // Existing graph geometry and out-of-canvas behavior do not change.
        svg.content.push_str(&format!(
            r#"<svg x="0" y="{header_height}" width="{width}" height="{height}" viewBox="0 0 {width} {height}" overflow="hidden">"#
        ));
        svg.content.push('\n');
        svg.render_graph(layout, routes, (width, height))?;
        svg.content.push_str("</svg>\n");
        svg.end_svg();
        Ok(svg.content)
    }

    fn render_graph(
        &mut self,
        layout: &LayoutResult,
        routes: Option<&RouteSet>,
        (width, height): (f32, f32),
    ) -> Result<()> {
        self.render_node_recursive(&layout.root)?;
        if let Some(routes) = routes {
            self.render_routes_in_viewport(routes, width, height)?;
        }
        Ok(())
    }

    fn render_background(&mut self) {
        self.content.push_str(&format!(
            r#"<rect x="0" y="0" width="{}" height="{}" fill="white" />"#,
            self.width, self.height
        ));
        self.content.push('\n');
    }

    fn render_fixed_rects(&mut self, node: &FixedNode, draw_leaves: bool) -> Result<()> {
        if node.children.is_empty() == draw_leaves {
            let fill = paint_to_css(&node.fill)?;
            let stroke = paint_to_css(&node.border)?;
            let [x, y, w, h] = node.rect;
            let dash = node.border_dash.as_deref().map(dash_css);
            self.render_styled_rect(
                x,
                y,
                w,
                h,
                &fill,
                &stroke,
                node.border_width,
                node.border_radius,
                dash.as_deref(),
            );
        }
        for child in &node.children {
            self.render_fixed_rects(child, draw_leaves)?;
        }
        Ok(())
    }

    fn render_fixed_texts(&mut self, node: &FixedNode) -> Result<()> {
        if node.text_runs.is_empty() && !node.label.is_empty() {
            let [x, y, w, h] = node.rect;
            self.render_text(
                x + w / 2.0,
                y + h / 2.0,
                &node.label,
                &Color::BLACK,
                14.0,
                "middle",
            );
        }
        for text in &node.text_runs {
            self.render_fixed_text(text)?;
        }
        for child in &node.children {
            self.render_fixed_texts(child)?;
        }
        Ok(())
    }

    fn render_fixed_text(&mut self, run: &TextRun) -> Result<()> {
        self.content.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"{}\" font-weight=\"{}\" font-family=\"PingFang SC, Hiragino Sans GB, Noto Sans CJK SC, Microsoft YaHei, sans-serif\" xml:space=\"preserve\">{}</text>\n",
            run.rect[0], run.baseline_y, paint_to_css(&run.color)?, run.font_size,
            run.font_weight.unwrap_or(400), escape_xml(&run.text)
        ));
        Ok(())
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

        self.write_definitions(0);

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

    fn write_definitions(&mut self, start: usize) {
        let definitions = &self.definitions[start..];
        if definitions.is_empty() {
            return;
        }
        self.content.push_str("<defs>\n");
        for def in definitions {
            self.content.push_str(def);
            self.content.push('\n');
        }
        self.content.push_str("</defs>\n");
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

        self.render_styled_rect(x, y, width, height, &fill, &stroke, stroke_width, rx, None);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_styled_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: &str,
        stroke: &str,
        stroke_width: f32,
        rx: f32,
        dash: Option<&str>,
    ) {
        let mut attrs = if rx > 0.0 {
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
        if let Some(dash) = dash {
            attrs.push_str(&format!(r#" stroke-dasharray="{}""#, dash));
        }
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
        self.render_text_row(x, y, text, color, font_size, anchor, false);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_text_row(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        color: &Color,
        font_size: f32,
        anchor: &str,
        preserve_space: bool,
    ) {
        let fill = color_to_css(color);
        let escaped_text = escape_xml(text);
        let whitespace = if preserve_space {
            r#" xml:space="preserve""#
        } else {
            ""
        };

        self.content.push_str(&format!(
            r#"<text x="{}" y="{}" fill="{}" font-size="{}" text-anchor="{}" class="label" dominant-baseline="middle"{}>{}</text>"#,
            x, y, fill, font_size, anchor, whitespace, escaped_text
        ));
        self.content.push('\n');
    }

    fn render_styled_polyline(
        &mut self,
        points: &[(f32, f32)],
        stroke: &str,
        stroke_width: f32,
        dash_array: Option<&str>,
        marker_start: Option<&str>,
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

        let mut attrs = format!(
            r#"points="{}" fill="none" stroke="{}" stroke-width="{}" class="edge""#,
            points_str, stroke, stroke_width
        );

        if let Some(dash) = dash_array {
            attrs.push_str(&format!(r#" stroke-dasharray="{}""#, dash));
        }

        if let Some(marker) = marker_start {
            attrs.push_str(&format!(r#" marker-start="url(#{})""#, marker));
        }

        if let Some(marker) = marker_end {
            attrs.push_str(&format!(r#" marker-end="url(#{})""#, marker));
        }

        self.content.push_str(&format!("<polyline {} />\n", attrs));
    }

    fn add_arrow_marker_with_orientation(&mut self, color: &Color, orientation: &str) -> String {
        self.marker_id_counter += 1;
        let id = format!("arrow-{}", self.marker_id_counter);
        let stroke = color_to_css(color);

        let marker = format!(
            r#"<marker id="{}" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="{}" markerUnits="strokeWidth">
                <path d="M0,0 L0,6 L9,3 z" fill="{}" />
            </marker>"#,
            id, orientation, stroke
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
                let text_x = if is_leaf {
                    rect.x() + rect.width() / 2.0
                } else {
                    rect.x() + 10.0
                };
                let text_y = if is_leaf {
                    rect.y() + rect.height() / 2.0
                } else {
                    rect.y() + 15.0
                };
                let anchor = if is_leaf { "middle" } else { "start" };
                if is_leaf && node.label.contains('\n') {
                    let count = node_label_lines(&node.label).count();
                    let first_y = text_y - (count - 1) as f32 * NODE_LABEL_LINE_HEIGHT / 2.0;
                    for (index, line) in node_label_lines(&node.label).enumerate() {
                        self.render_text_row(
                            text_x,
                            first_y + index as f32 * NODE_LABEL_LINE_HEIGHT,
                            line,
                            &node.style.text_color,
                            14.0,
                            anchor,
                            true,
                        );
                    }
                } else {
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
        }

        // Render children (whether or not this node is a container)
        for child in &node.children {
            self.render_node_recursive(child)?;
        }

        Ok(())
    }

    /// Render a route
    fn render_route(
        &mut self,
        route: &Route,
        marker: &str,
        label_position: Option<(f32, f32, f32)>,
    ) {
        let points: Vec<(f32, f32)> = route.segments.iter().map(|s| (s.x, s.y)).collect();

        // Use edge_color and edge_kind from route
        let color = &route.edge_color;
        let stroke_width = route.edge_kind.stroke_width();
        let dash_array = route.edge_kind.stroke_dasharray();

        self.render_styled_polyline(
            &points,
            &color_to_css(color),
            stroke_width,
            dash_array.as_deref(),
            (route.edge_kind == EdgeKind::Bidirectional).then_some(marker),
            Some(marker),
        );

        if let (Some(label), Some((x, y, font))) = (&route.label, label_position) {
            self.render_text(x, y, label, color, font, "middle");
        }
    }

    fn render_routes_in_viewport(
        &mut self,
        routes: &RouteSet,
        width: f32,
        height: f32,
    ) -> Result<()> {
        // Resolve all positions before writing any of the routes. Fixed scenes retain
        // their caller-supplied text positions and never enter this path.
        let routes = routes
            .routes()
            .iter()
            .filter(|route| !route.segments.is_empty())
            .map(|route| Ok((route, route_label_position(route, width, height)?)))
            .collect::<Result<Vec<_>>>()?;
        let first_new_def = self.definitions.len();
        let routes: Vec<_> = routes
            .into_iter()
            .map(|(route, label_position)| {
                let orient = if route.edge_kind == EdgeKind::Bidirectional {
                    "auto-start-reverse"
                } else {
                    "auto"
                };
                (
                    route,
                    self.add_arrow_marker_with_orientation(&route.edge_color, orient),
                    label_position,
                )
            })
            .collect();

        // Mount the actual markers before any polyline references them.
        self.write_definitions(first_new_def);
        for (route, marker, label_position) in routes {
            self.render_route(route, &marker, label_position);
        }
        Ok(())
    }
}

impl Backend for SvgBackend {
    fn render(&mut self, layout: &LayoutResult, routes: Option<&RouteSet>) -> Result<()> {
        self.content.clear();
        self.definitions.clear();
        self.marker_id_counter = 0;
        self.start_svg();
        self.render_background();
        self.render_graph(layout, routes, (self.width, self.height))?;
        self.end_svg();
        Ok(())
    }

    fn render_node(&mut self, node: &LayoutNode) -> Result<()> {
        self.render_node_recursive(node)
    }

    fn render_routes(&mut self, routes: &RouteSet) -> Result<()> {
        self.render_routes_in_viewport(routes, self.width, self.height)
    }

    fn output(&self) -> Option<String> {
        Some(self.content.clone())
    }

    fn save(&self, path: &str) -> Result<()> {
        std::fs::write(path, &self.content).map_err(|e| TectonError::IoError(e))?;
        Ok(())
    }
}

/// Choose a nearby readable position without modifying the route.
/// SVG fonts are selected by the viewer, so reserve a conservative em envelope,
/// including for CJK, rather than promising a font-specific ink measurement.
fn route_label_position(route: &Route, width: f32, height: f32) -> Result<Option<(f32, f32, f32)>> {
    let Some(label) = route
        .label
        .as_deref()
        .filter(|label| !label.trim().is_empty())
    else {
        return Ok(None);
    };
    if route.segments.len() < 2 {
        return Ok(None);
    }
    const PADDING: f32 = 6.0;
    const FONT_MAX: f32 = 12.0;
    const FONT_MIN: f32 = 8.0;
    let ems = label.chars().count() as f32 * 1.25;
    let font = FONT_MAX
        .min((width - 2.0 * PADDING) / ems)
        .min((height - 2.0 * PADDING) / 2.0);
    if !font.is_finite() || font < FONT_MIN {
        return Err(TectonError::invalid_input(
            "edge label cannot fit the automatic SVG viewport",
        ));
    }
    let inset_x = PADDING + font * ems / 2.0;
    let inset_y = PADDING + font;
    // Protect against rounding up when the em envelope exactly fills the viewport.
    let right = (width - inset_x).max(inset_x);
    let bottom = (height - inset_y).max(inset_y);
    let mut best = None::<(f32, f32, f32, f32)>;
    for pair in route.segments.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
        let (mid_x, mid_y) = ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        let (horizontal, len) = (dx >= dy, dx + dy);
        let offsets = if horizontal {
            [(0.0, -font - 2.0), (0.0, font + 2.0)]
        } else {
            [
                (font * ems / 2.0 + 6.0, 0.0),
                (-font * ems / 2.0 - 6.0, 0.0),
            ]
        };
        for (ox, oy) in offsets {
            let (raw_x, raw_y) = (mid_x + ox, mid_y + oy);
            let (x, y) = (raw_x.clamp(inset_x, right), raw_y.clamp(inset_y, bottom));
            let score = (raw_x - x).abs() + (raw_y - y).abs() + if horizontal { 0.0 } else { font };
            if !score.is_finite() {
                continue;
            }
            if best.map_or(true, |(old_score, old_len, _, _)| {
                score < old_score || (score == old_score && len > old_len)
            }) {
                best = Some((score, len, x, y));
            }
        }
    }
    Ok(best.map(|(_, _, x, y)| (x, y, font)))
}

fn paint_to_css(value: &str) -> Result<String> {
    Ok(paint(value)?.map_or_else(|| "none".to_owned(), |color| color_to_css(&color)))
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
        assert!(output.contains("<polyline") && output.contains("<defs>"));
        backend.render(&layout, Some(&route_set)).unwrap();
        assert_eq!(backend.output().unwrap(), output);
        backend.render(&layout, None).unwrap();
        let no_routes = backend.output().unwrap();
        assert!(!no_routes.contains("<polyline") && !no_routes.contains("<marker"));
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
