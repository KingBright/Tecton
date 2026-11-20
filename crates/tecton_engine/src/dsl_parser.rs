use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tecton_types::{
    edge::{Edge, EdgeId, EdgeKind},
    geometry::{Point, Rect, Size},
    id::{NodeId, PortId},
    style::{Color, FlexDirection, Margin, Padding, Style},
};

use crate::{
    layout_graph::{LayoutGraph, LayoutNode, LayoutPort},
    named_layout::{BoundEdge, DiagramBindings},
};

/// DSL representation of a diagram (JSON/YAML compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramDsl {
    /// Canvas dimensions
    #[serde(default = "default_canvas_size")]
    pub canvas: CanvasSpec,
    /// Nodes in the diagram
    #[serde(default)]
    pub nodes: Vec<NodeSpec>,
    /// Edges connecting nodes
    #[serde(default)]
    pub edges: Vec<EdgeSpec>,
    /// Global styles
    #[serde(default)]
    pub styles: HashMap<String, StyleSpec>,
}

fn default_canvas_size() -> CanvasSpec {
    CanvasSpec {
        width: 800.0,
        height: 600.0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSpec {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSpec {
    /// Node ID (optional, auto-generated if not provided)
    #[serde(default)]
    pub id: Option<String>,
    /// Node label
    pub label: String,
    /// Node type/style reference
    #[serde(default)]
    pub style: Option<String>,
    /// Inline style properties
    #[serde(flatten)]
    pub inline_style: Option<InlineStyleSpec>,
    /// Child nodes (for containers)
    #[serde(default)]
    pub children: Vec<NodeSpec>,
    /// Ports on this node
    #[serde(default)]
    pub ports: Vec<PortSpec>,
    /// Flex direction for children
    #[serde(default)]
    pub flex_direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineStyleSpec {
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub border_color: Option<String>,
    #[serde(default)]
    pub border_width: Option<f32>,
    #[serde(default)]
    pub border_radius: Option<f32>,
    #[serde(default)]
    pub text_color: Option<String>,
    #[serde(default)]
    pub font_size: Option<f32>,
    #[serde(default)]
    pub padding: Option<f32>,
    #[serde(default)]
    pub margin: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortSpec {
    pub name: String,
    pub side: String,
    #[serde(default = "default_port_position")]
    pub position: f32,
}

fn default_port_position() -> f32 {
    0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSpec {
    /// Edge ID (optional)
    #[serde(default)]
    pub id: Option<String>,
    /// Source port reference (format: "node_id:port_name")
    pub from: String,
    /// Target port reference (format: "node_id:port_name")
    pub to: String,
    /// Edge kind/style
    #[serde(default)]
    pub kind: String,
    /// Edge color
    #[serde(default)]
    pub color: Option<String>,
    /// Edge label
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleSpec {
    pub background: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<f32>,
    pub border_radius: Option<f32>,
    pub text_color: Option<String>,
    pub font_size: Option<f32>,
    pub padding: Option<f32>,
    pub margin: Option<f32>,
}

/// Parser for DSL input
pub struct DslParser;

impl DslParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse JSON input
    pub fn parse_json(&self, json: &str) -> Result<DiagramDsl, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    /// Parse YAML input
    pub fn parse_yaml(&self, yaml: &str) -> Result<DiagramDsl, String> {
        serde_yaml::from_str(yaml).map_err(|e| e.to_string())
    }

    /// Convert DSL to LayoutGraph
    pub fn to_layout_graph(&self, dsl: &DiagramDsl) -> Result<LayoutGraph, String> {
        self.build_layout_graph(dsl).map(|(graph, _)| graph)
    }

    /// Compile explicitly named graph input, rejecting ambiguous bindings before layout.
    pub fn to_layout_graph_with_bindings(
        &self,
        dsl: &DiagramDsl,
    ) -> Result<(LayoutGraph, DiagramBindings), String> {
        fn validate_nodes<'a>(
            nodes: &'a [NodeSpec],
            names: &mut HashSet<&'a str>,
        ) -> Result<(), String> {
            for node in nodes {
                let id = node
                    .id
                    .as_deref()
                    .ok_or("every bound node needs an explicit ID")?;
                if id.trim().is_empty() || id.contains(':') || !names.insert(id) {
                    return Err(format!("empty, invalid or duplicate bound node ID {id:?}"));
                }
                let mut ports = HashSet::new();
                for port in &node.ports {
                    let name = port.name.as_str();
                    if name.trim().is_empty() || name.contains(':') || !ports.insert(name) {
                        return Err(format!(
                            "empty, invalid or duplicate port {name:?} on {id:?}"
                        ));
                    }
                }
                validate_nodes(&node.children, names)?;
            }
            Ok(())
        }
        validate_nodes(&dsl.nodes, &mut HashSet::new())?;
        let mut edges = HashSet::new();
        for edge in &dsl.edges {
            let id = edge
                .id
                .as_deref()
                .ok_or("every bound edge needs an explicit ID")?;
            if id.trim().is_empty() || !edges.insert(id) {
                return Err(format!("empty or duplicate bound edge ID {id:?}"));
            }
        }
        self.build_layout_graph(dsl)
    }

    fn build_layout_graph(
        &self,
        dsl: &DiagramDsl,
    ) -> Result<(LayoutGraph, DiagramBindings), String> {
        let mut graph = LayoutGraph::new();
        let mut node_id_map: HashMap<String, NodeId> = HashMap::new();
        let mut port_id_map: HashMap<String, PortId> = HashMap::new();
        let mut node_counter: usize = 0;

        // Process nodes
        for node_spec in &dsl.nodes {
            node_counter += 1;
            self.process_node(
                node_spec,
                &mut graph,
                &mut node_id_map,
                &mut port_id_map,
                None,
                &dsl.styles,
                node_counter,
            )?;
        }

        let mut edge_id_map = HashMap::new();
        for edge_spec in &dsl.edges {
            let edge = self.process_edge(edge_spec, &mut graph, &port_id_map)?;
            if let Some(id) = &edge_spec.id {
                edge_id_map.insert(id.clone(), edge);
            }
        }

        Ok((
            graph,
            DiagramBindings {
                nodes: node_id_map,
                ports: port_id_map,
                edges: edge_id_map,
            },
        ))
    }

    fn process_node(
        &self,
        spec: &NodeSpec,
        graph: &mut LayoutGraph,
        node_id_map: &mut HashMap<String, NodeId>,
        port_id_map: &mut HashMap<String, PortId>,
        parent_id: Option<NodeId>,
        styles: &HashMap<String, StyleSpec>,
        node_counter: usize,
    ) -> Result<NodeId, String> {
        // Generate or use provided ID
        let node_id = NodeId::new();
        let node_key = spec
            .id
            .clone()
            .unwrap_or_else(|| format!("node_{}", node_counter));

        // Build style
        let style = self.build_style(spec, styles)?;

        // Create node
        let mut node = LayoutNode::new(node_id, &spec.label).with_style(style);

        if let Some(parent) = parent_id {
            node = node.with_parent(parent);
        }

        // Add ports
        for port_spec in &spec.ports {
            let port_id = PortId::new();
            let port_key = format!("{}:{}", node_key, port_spec.name);

            let side = parse_port_side(&port_spec.side)?;
            let port =
                LayoutPort::new(port_id, &port_spec.name, side).with_position(port_spec.position);

            node.add_port(port);
            port_id_map.insert(port_key, port_id);
        }

        // Add to graph
        let actual_id = graph.add_node(node);
        node_id_map.insert(node_key.clone(), actual_id);

        // Process children
        let mut child_counter = node_counter;
        for child_spec in &spec.children {
            child_counter += 1;
            let child_id = self.process_node(
                child_spec,
                graph,
                node_id_map,
                port_id_map,
                Some(actual_id),
                styles,
                child_counter,
            )?;

            // Add child to parent
            if let Some(parent) = graph.get_node_mut(actual_id) {
                parent.add_child(child_id);
            }
        }

        Ok(actual_id)
    }

    fn build_style(
        &self,
        spec: &NodeSpec,
        styles: &HashMap<String, StyleSpec>,
    ) -> Result<Style, String> {
        let mut style = Style::default();

        // Apply referenced style first
        if let Some(style_ref) = &spec.style {
            if let Some(style_spec) = styles.get(style_ref) {
                style = apply_style_spec(style, style_spec)?;
            }
        }

        // Apply inline styles
        if let Some(inline) = &spec.inline_style {
            if let Some(bg) = &inline.background {
                style.background =
                    Color::from_hex(bg).ok_or_else(|| format!("Invalid color: {}", bg))?;
            }
            if let Some(bc) = &inline.border_color {
                style.border_color =
                    Color::from_hex(bc).ok_or_else(|| format!("Invalid color: {}", bc))?;
            }
            if let Some(bw) = inline.border_width {
                style.border_width = bw.into();
            }
            if let Some(br) = inline.border_radius {
                style.border_radius = br.into();
            }
            if let Some(p) = inline.padding {
                style.padding = Padding::uniform(p);
            }
            if let Some(m) = inline.margin {
                style.margin = Margin::uniform(m);
            }
        }

        // Apply flex direction
        if let Some(fd) = &spec.flex_direction {
            style.flex_direction = parse_flex_direction(fd)?;
        }

        Ok(style)
    }

    fn process_edge(
        &self,
        spec: &EdgeSpec,
        graph: &mut LayoutGraph,
        port_id_map: &HashMap<String, PortId>,
    ) -> Result<BoundEdge, String> {
        // Parse port references
        let source_port_id = port_id_map
            .get(&spec.from)
            .ok_or_else(|| format!("Source port not found: {}", spec.from))?;
        let target_port_id = port_id_map
            .get(&spec.to)
            .ok_or_else(|| format!("Target port not found: {}", spec.to))?;

        let mut edge = Edge::new(EdgeId::new(), *source_port_id, *target_port_id);

        // Apply edge kind
        edge.kind = parse_edge_kind(&spec.kind);

        // Apply color
        if let Some(color_str) = &spec.color {
            edge.color = Color::from_hex(color_str)
                .ok_or_else(|| format!("Invalid color: {}", color_str))?;
        }

        // Apply label
        if let Some(label) = &spec.label {
            edge.label = Some(label.clone());
        }

        let binding = BoundEdge {
            edge_id: edge.id,
            source_port: edge.source_port,
            target_port: edge.target_port,
        };
        graph.add_edge(edge)?;
        Ok(binding)
    }
}

impl Default for DslParser {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_port_side(side: &str) -> Result<tecton_types::geometry::PortSide, String> {
    match side.to_lowercase().as_str() {
        "north" | "top" | "n" => Ok(tecton_types::geometry::PortSide::North),
        "south" | "bottom" | "s" => Ok(tecton_types::geometry::PortSide::South),
        "east" | "right" | "e" => Ok(tecton_types::geometry::PortSide::East),
        "west" | "left" | "w" => Ok(tecton_types::geometry::PortSide::West),
        _ => Err(format!("Invalid port side: {}", side)),
    }
}

fn parse_flex_direction(dir: &str) -> Result<FlexDirection, String> {
    match dir.to_lowercase().as_str() {
        "row" => Ok(FlexDirection::Row),
        "column" | "col" => Ok(FlexDirection::Column),
        "row-reverse" => Ok(FlexDirection::RowReverse),
        "column-reverse" | "col-reverse" => Ok(FlexDirection::ColumnReverse),
        _ => Err(format!("Invalid flex direction: {}", dir)),
    }
}

fn parse_edge_kind(kind: &str) -> EdgeKind {
    match kind.to_lowercase().as_str() {
        "dashed" => EdgeKind::Dashed,
        "dotted" => EdgeKind::Dotted,
        "bold" | "thick" => EdgeKind::Bold,
        "bidirectional" | "bi" => EdgeKind::Bidirectional,
        _ => EdgeKind::Default,
    }
}

fn apply_style_spec(style: Style, spec: &StyleSpec) -> Result<Style, String> {
    let mut result = style;

    if let Some(bg) = &spec.background {
        result.background = Color::from_hex(bg).ok_or_else(|| format!("Invalid color: {}", bg))?;
    }
    if let Some(bc) = &spec.border_color {
        result.border_color =
            Color::from_hex(bc).ok_or_else(|| format!("Invalid color: {}", bc))?;
    }
    if let Some(bw) = spec.border_width {
        result.border_width = bw.into();
    }
    if let Some(br) = spec.border_radius {
        result.border_radius = br.into();
    }
    if let Some(p) = spec.padding {
        result.padding = Padding::uniform(p);
    }
    if let Some(m) = spec.margin {
        result.margin = Margin::uniform(m);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        let json = r#"{
            "canvas": {"width": 800, "height": 600},
            "nodes": [
                {"label": "Node 1", "ports": [{"name": "out", "side": "east"}]},
                {"label": "Node 2", "ports": [{"name": "in", "side": "west"}]}
            ],
            "edges": [
                {"from": "node_1:out", "to": "node_2:in"}
            ]
        }"#;

        let parser = DslParser::new();
        let dsl = parser.parse_json(json).unwrap();

        assert_eq!(dsl.canvas.width, 800.0);
        assert_eq!(dsl.nodes.len(), 2);
    }

    #[test]
    fn test_port_side_parsing() {
        assert!(matches!(
            parse_port_side("north").unwrap(),
            tecton_types::geometry::PortSide::North
        ));
        assert!(matches!(
            parse_port_side("EAST").unwrap(),
            tecton_types::geometry::PortSide::East
        ));
        assert!(parse_port_side("invalid").is_err());
    }
}
