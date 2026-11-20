//! Positioned native SVG input, separate from the automatic layout pipeline.
use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use tecton_types::{
    error::{Result, TectonError},
    style::Color,
};

#[derive(Debug, Deserialize)]
pub struct FixedScene {
    pub canvas: [f32; 2],
    pub nodes: Vec<FixedNode>,
    pub edges: Vec<FixedRoute>,
    #[serde(default)]
    pub annotations: Vec<TextRun>,
    #[serde(default)]
    pub logical_edges: Vec<LogicalEdge>,
}

#[derive(Debug, Deserialize)]
pub struct FixedNode {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub rect: [f32; 4],
    pub fill: String,
    pub border: String,
    pub border_width: f32,
    pub border_radius: f32,
    pub border_dash: Option<Vec<f32>>,
    #[serde(default)]
    pub text_runs: Vec<TextRun>,
    #[serde(default)]
    pub children: Vec<FixedNode>,
}

#[derive(Debug, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub rect: [f32; 4],
    pub font_size: f32,
    pub color: String,
    pub baseline_y: f32,
    pub font_weight: Option<u16>,
    // Metadata only. Rendering never opens any path supplied by input.
    pub font_file: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    Solid,
    Dashed,
}

#[derive(Debug, Deserialize)]
pub struct FixedRoute {
    pub id: String,
    pub points: Vec<[f32; 2]>,
    pub color: String,
    pub width: f32,
    pub kind: LineKind,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub dash: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
pub struct LogicalEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub routes: Vec<String>,
    pub bidirectional: bool,
}

pub(crate) fn paint(value: &str) -> Result<Option<Color>> {
    if value == "none" {
        return Ok(None);
    }
    let hex = value.strip_prefix('#').unwrap_or_default();
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(TectonError::invalid_input(
            "expected #RRGGBB, #RRGGBBAA or none",
        ));
    }
    Ok(Color::from_hex(value))
}

fn invalid(label: &str, what: &str) -> TectonError {
    TectonError::invalid_input(format!("{label}: {what}"))
}

fn rect_ok([x, y, w, h]: [f32; 4]) -> bool {
    [x, y, w, h, x + w, y + h].iter().all(|n| n.is_finite()) && w >= 0.0 && h >= 0.0
}

fn dash_ok(dash: Option<&[f32]>) -> bool {
    dash.map_or(true, |d| {
        !d.is_empty() && d.iter().all(|v| v.is_finite() && *v > 0.0)
    })
}

fn xml_text_safe(text: &str) -> bool {
    text.chars().all(|ch| {
        matches!(
            u32::from(ch),
            0x9 | 0xa | 0xd | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff
        )
    })
}

fn check_text(run: &TextRun) -> Result<()> {
    paint(&run.color)?;
    if !rect_ok(run.rect)
        || !(run.font_size.is_finite() && run.font_size > 0.0)
        || !run.baseline_y.is_finite()
        || !run.font_weight.map_or(true, |w| (1..=1000).contains(&w))
        || !xml_text_safe(&run.text)
    {
        return Err(TectonError::invalid_input("invalid fixed text run"));
    }
    Ok(())
}

fn collect_node<'a>(node: &'a FixedNode, nodes: &mut HashMap<&'a str, [f32; 4]>) -> Result<()> {
    if node.id.is_empty() || nodes.insert(&node.id, node.rect).is_some() {
        return Err(invalid(&node.id, "empty or duplicate node ID"));
    }
    paint(&node.fill)?;
    paint(&node.border)?;
    if !rect_ok(node.rect)
        || node.rect[2] == 0.0
        || node.rect[3] == 0.0
        || !(node.border_width.is_finite() && node.border_width >= 0.0)
        || !(node.border_radius.is_finite() && node.border_radius >= 0.0)
        || !dash_ok(node.border_dash.as_deref())
    {
        return Err(invalid(&node.id, "invalid node geometry"));
    }
    if node.text_runs.is_empty() && !xml_text_safe(&node.label) {
        return Err(invalid(&node.id, "invalid fixed node label"));
    }
    for text in &node.text_runs {
        check_text(text)?;
    }
    for child in &node.children {
        collect_node(child, nodes)?;
    }
    Ok(())
}

fn on_boundary([x, y]: [f32; 2], [left, top, w, h]: [f32; 4]) -> bool {
    let (right, bottom) = (left + w, top + h);
    let epsilon = 0.001;
    let near = |a: f32, b: f32| (a - b).abs() <= epsilon;
    let within = |v: f32, low: f32, high: f32| v >= low - epsilon && v <= high + epsilon;
    ((near(x, left) || near(x, right)) && within(y, top, bottom))
        || ((near(y, top) || near(y, bottom)) && within(x, left, right))
}

impl FixedScene {
    pub fn validate(&self) -> Result<()> {
        if !self.canvas.iter().all(|n| n.is_finite() && *n > 0.0) {
            return Err(TectonError::invalid_input(
                "invalid fixed canvas dimensions",
            ));
        }
        let mut nodes = HashMap::new();
        for node in &self.nodes {
            collect_node(node, &mut nodes)?;
        }
        for text in &self.annotations {
            check_text(text)?;
        }
        let mut routes = HashMap::new();
        for route in &self.edges {
            if route.id.is_empty() || routes.insert(route.id.as_str(), route).is_some() {
                return Err(invalid(&route.id, "empty or duplicate visual route ID"));
            }
            paint(&route.color)?;
            if route.points.len() < 2
                || !route.points.iter().flatten().all(|v| v.is_finite())
                || route.points.windows(2).any(|pair| pair[0] == pair[1])
                || !(route.width.is_finite() && route.width > 0.0)
                || !dash_ok(route.dash.as_deref())
            {
                return Err(invalid(&route.id, "invalid visual route geometry"));
            }
        }
        let mut logical_ids = HashSet::new();
        for edge in &self.logical_edges {
            if edge.id.is_empty() || !logical_ids.insert(&edge.id) || edge.routes.is_empty() {
                return Err(invalid(&edge.id, "empty or duplicate logical edge"));
            }
            let from = nodes
                .get(edge.source.as_str())
                .ok_or_else(|| invalid(&edge.id, "unknown source"))?;
            let to = nodes
                .get(edge.target.as_str())
                .ok_or_else(|| invalid(&edge.id, "unknown target"))?;
            let mut seen = HashSet::new();
            let mut chain = Vec::with_capacity(edge.routes.len());
            for id in &edge.routes {
                if !seen.insert(id) {
                    return Err(invalid(&edge.id, "route repeated within chain"));
                }
                chain.push(
                    *routes
                        .get(id.as_str())
                        .ok_or_else(|| invalid(&edge.id, "unknown route"))?,
                );
            }
            let first = chain[0];
            let last = chain[chain.len() - 1];
            if !on_boundary(first.points[0], *from)
                || !on_boundary(last.points[last.points.len() - 1], *to)
            {
                return Err(invalid(&edge.id, "route endpoint is off its node boundary"));
            }
            if first.arrow_start != edge.bidirectional || (edge.bidirectional && !last.arrow_end) {
                return Err(invalid(&edge.id, "inconsistent logical arrow direction"));
            }
            for pair in chain.windows(2) {
                if pair[0].points.last() != pair[1].points.first() {
                    return Err(invalid(&edge.id, "disconnected route chain"));
                }
                if pair[0].arrow_end || pair[1].arrow_start {
                    return Err(invalid(&edge.id, "internal junction arrow"));
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn dash_css(dash: &[f32]) -> String {
    dash.iter()
        .map(f32::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}
