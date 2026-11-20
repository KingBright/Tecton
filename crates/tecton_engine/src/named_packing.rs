//! Intrinsic, ordered packing for strict named diagrams. No input has coordinates.
use tecton_layout::layout_context::{
    node_label_lines, LayoutNode, LayoutResult, NODE_LABEL_LINE_HEIGHT,
};
use tecton_types::{
    error::{Result, TectonError},
    geometry::Rect,
    id::NodeId,
    style::Style,
};

use crate::{layout_graph::LayoutGraph, orchestrator::OrchestratorConfig};

const MAX_AXIS: f32 = 2_600.0;
const MAX_CELLS: f32 = 32_000.0;
const MAX_ROUTE_WORK: f32 = 2_000_000.0;
const MAX_NODES: usize = 256;
const MAX_DEPTH: usize = 24;

/// Named cards use 14px system text. Count Unicode scalars instead of bytes
/// and reserve em-sized room for wide CJK/emoji; this is an estimate, not
/// font-specific glyph measurement. Rows and their order are never changed.
fn text_extent(label: &str) -> (f32, usize) {
    node_label_lines(label).fold((0.0_f32, 0), |(max_width, rows), line| {
        let width = line
            .chars()
            .map(|c| match c {
                '\t' => 32.0,
                'i' | 'l' | 'I' | '.' | ',' | ':' | '!' | ' ' => 6.0,
                'W' | 'M' | 'w' | 'm' => 14.0,
                c if c.is_ascii() => 10.0,
                _ => 16.0,
            })
            .sum();
        (max_width.max(width), rows + 1)
    })
}

/// Reserve routing lanes between boxes, and limit shelves by measured width.
fn place(children: &mut [LayoutNode], max_width: f32, gap: f32, column: bool) -> (f32, f32) {
    let (mut x, mut y, mut row_height, mut right): (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.0);
    for child in children {
        let (w, h) = (child.rect.width(), child.rect.height());
        if x > 0.0 && (column || x + w > max_width) {
            y += row_height + gap;
            x = 0.0;
            row_height = 0.0;
        }
        translate(child, x, y);
        right = right.max(x + w);
        row_height = row_height.max(h);
        x += w + gap;
    }
    (right, y + row_height)
}

fn translate(node: &mut LayoutNode, x: f32, y: f32) {
    let r = node.rect;
    node.rect = Rect::new(r.x() + x, r.y() + y, r.width(), r.height());
    for child in &mut node.children {
        translate(child, x, y);
    }
}

fn pack_node(
    graph: &LayoutGraph,
    id: NodeId,
    width: f32,
    gap: f32,
    depth: usize,
) -> Result<LayoutNode> {
    if depth > MAX_DEPTH {
        return Err(TectonError::layout(
            "named diagram groups are nested too deeply",
        ));
    }
    let source = graph
        .get_node(id)
        .ok_or_else(|| TectonError::layout("missing group child"))?;
    let (ink, rows) = text_extent(&source.label);
    let (mut node_width, mut node_height) = (
        (ink + 40.0).max(66.0),
        50.0 + (rows - 1) as f32 * NODE_LABEL_LINE_HEIGHT,
    );
    let mut children = Vec::new();
    if !source.children.is_empty() {
        let (left, right, top, bottom) = (
            source.style.padding.left().max(gap / 2.0),
            source.style.padding.right().max(gap / 2.0),
            source.style.padding.top().max(gap / 2.0)
                + if source.label.is_empty() { 0.0 } else { 20.0 },
            source.style.padding.bottom().max(gap / 2.0),
        );
        let child_width = (width - left - right).max(0.0);
        for &child in &source.children {
            children.push(pack_node(graph, child, child_width, gap, depth + 1)?);
        }
        if source.style.flex_direction.is_reverse() {
            children.reverse();
        }
        let gap = gap.max(source.style.gap());
        let (body_width, body_height) = place(
            &mut children,
            child_width,
            gap,
            source.style.flex_direction.is_column(),
        );
        for child in &mut children {
            translate(child, left, top);
        }
        node_width = node_width.max(left + body_width + right);
        node_height = top + body_height + bottom;
    }
    if !node_width.is_finite()
        || !node_height.is_finite()
        || node_width > MAX_AXIS
        || node_height > MAX_AXIS
    {
        return Err(TectonError::layout(
            "named node is too large to pack without clipping",
        ));
    }
    let mut node = LayoutNode::new(
        id,
        Rect::new(0.0, 0.0, node_width, node_height),
        source.style.clone(),
    )
    .with_label(source.label.clone());
    node.children = children;
    Ok(node)
}

pub(crate) fn pack_named_forest(
    graph: &LayoutGraph,
    config: &OrchestratorConfig,
) -> Result<LayoutResult> {
    let nodes = graph.nodes();
    let edges = graph.edges().len();
    let cell = config.routing_cell_size;
    let requested = config.canvas_bounds;
    if nodes.len() > MAX_NODES
        || !cell.is_finite()
        || cell < 1.0
        || cell > 60.0
        || [requested.width(), requested.height()]
            .iter()
            .any(|n| !n.is_finite() || *n <= 0.0 || *n > MAX_AXIS)
    {
        return Err(TectonError::layout(
            "named diagram exceeds the bounded packing input",
        ));
    }
    let outer = 3.0 * cell;
    let gap = 4.0 * cell;
    let width_budget = (requested.width() - 2.0 * outer).max(0.0);
    let mut children = Vec::new();
    for node in nodes.into_iter().filter(|n| n.parent.is_none()) {
        children.push(pack_node(graph, node.id, width_budget, gap, 0)?);
    }
    let (used_width, used_height) = place(&mut children, width_budget, gap, false);
    let width = (used_width + 2.0 * outer).max(requested.width()).ceil();
    let height = (used_height + 2.0 * outer).max(requested.height()).ceil();
    let cells = (width / cell).ceil() * (height / cell).ceil();
    if width > MAX_AXIS
        || height > MAX_AXIS
        || cells > MAX_CELLS
        || cells * edges as f32 > MAX_ROUTE_WORK
    {
        return Err(TectonError::layout(
            "named drawing exceeds the bounded routing canvas",
        ));
    }

    let bounds = Rect::new(0.0, 0.0, width, height);
    for child in &mut children {
        translate(child, outer, outer);
    }
    let mut root = LayoutNode::new(NodeId::new(), bounds, Style::default());
    root.children = children;
    Ok(LayoutResult::new(root, bounds))
}
