//! Converter from Tecton DSL AST to LayoutGraph
//!
//! This module bridges the tecton_dsl parser with the tecton_engine,
//! converting the Pest-based DSL AST into a LayoutGraph.

use std::collections::HashMap;

use tecton_dsl::ast::{Container, ContainerKind, Edge, EdgeOp, Node, Stmt, TectonFile, Value};
use tecton_types::{
    geometry::PortSide,
    id::{NodeId, PortId},
    shape::ShapeType,
    style::{Color, FlexDirection, Margin, Padding, Style},
};

use crate::layout_graph::{LayoutGraph, LayoutNode, LayoutPort};

/// Converts a TectonFile (DSL AST) to a LayoutGraph
pub fn convert_to_layout_graph(file: &TectonFile) -> Result<LayoutGraph, String> {
    let mut graph = LayoutGraph::new();
    let mut node_id_map: HashMap<String, NodeId> = HashMap::new();
    let mut styles: HashMap<String, Style> = HashMap::new();

    // First pass: extract styles from classes
    for class in &file.classes {
        let style = convert_properties_to_style(&class.properties)?;
        styles.insert(class.name.clone(), style);
    }

    // Second pass: create nodes and containers
    for stmt in &file.statements {
        match stmt {
            Stmt::Container(container) => {
                convert_container(&mut graph, &mut node_id_map, container, &styles, None)?;
            }
            Stmt::Node(node) => {
                convert_node(&mut graph, &mut node_id_map, node, &styles)?;
            }
            _ => {} // Edge statements handled in third pass
        }
    }

    // Third pass: create edges
    for stmt in &file.statements {
        if let Stmt::Edge(edge) = stmt {
            convert_edge(&mut graph, &node_id_map, edge)?;
        }
    }

    Ok(graph)
}

fn convert_container(
    graph: &mut LayoutGraph,
    node_id_map: &mut HashMap<String, NodeId>,
    container: &Container,
    styles: &HashMap<String, Style>,
    parent_id: Option<NodeId>,
) -> Result<NodeId, String> {
    let container_id = NodeId::new();
    let container_name = format!("{:?}", container.kind);

    let mut style = convert_properties_to_style(&container.properties)?;

    // Provide sensible defaults for containers
    if !container.properties.contains_key("padding") {
        style.padding = Padding::uniform(20.0);
    }
    if !container.properties.contains_key("gap") {
        style.gap = ordered_float::OrderedFloat(20.0);
    }

    // Set flex direction based on container kind
    style.flex_direction = match container.kind {
        ContainerKind::Row => FlexDirection::Row,
        ContainerKind::Column => FlexDirection::Column,
        ContainerKind::Stack => FlexDirection::Row,
    };

    let mut layout_node = LayoutNode::new(container_id, &container_name).with_style(style);

    if let Some(parent) = parent_id {
        layout_node = layout_node.with_parent(parent);
    }

    // Add default ports
    layout_node.add_port(LayoutPort::new(PortId::new(), "north", PortSide::North));
    layout_node.add_port(LayoutPort::new(PortId::new(), "east", PortSide::East));
    layout_node.add_port(LayoutPort::new(PortId::new(), "south", PortSide::South));
    layout_node.add_port(LayoutPort::new(PortId::new(), "west", PortSide::West));

    graph.add_node(layout_node);
    node_id_map.insert(container_name.clone(), container_id);

    // Process children
    let mut child_ids = Vec::new();
    for child_stmt in &container.children {
        match child_stmt {
            Stmt::Container(child_container) => {
                let child_id = convert_container(
                    graph,
                    node_id_map,
                    child_container,
                    styles,
                    Some(container_id),
                )?;
                child_ids.push(child_id);
            }
            Stmt::Node(node) => {
                let node_id = convert_node(graph, node_id_map, node, styles)?;
                if let Some(layout_node) = graph.get_node_mut(node_id) {
                    layout_node.parent = Some(container_id);
                }
                child_ids.push(node_id);
            }
            _ => {}
        }
    }

    // Add children to container
    if let Some(layout_node) = graph.get_node_mut(container_id) {
        for child_id in child_ids {
            layout_node.add_child(child_id);
        }
    }

    Ok(container_id)
}

fn convert_node(
    graph: &mut LayoutGraph,
    node_id_map: &mut HashMap<String, NodeId>,
    node: &Node,
    styles: &HashMap<String, Style>,
) -> Result<NodeId, String> {
    let node_id = NodeId::new();
    let mut style = convert_properties_to_style(&node.properties)?;

    // Handle class reference
    if let Some(Value::Identifier(class_name)) = node.properties.get("class") {
        if let Some(class_style) = styles.get(class_name) {
            style = merge_styles(class_style.clone(), style);
        }
    }

    // Extract label
    let label = node
        .properties
        .get("label")
        .and_then(|v| v.as_string())
        .unwrap_or(&node.id)
        .to_string();

    // Handle shape
    if let Some(Value::Identifier(shape_name)) = node.properties.get("shape") {
        style.shape = parse_shape_type(shape_name);
    }

    let mut layout_node = LayoutNode::new(node_id, &label).with_style(style.clone());

    layout_node = layout_node.with_shape(style.shape);

    // Add default ports
    layout_node.add_port(LayoutPort::new(PortId::new(), "north", PortSide::North));
    layout_node.add_port(LayoutPort::new(PortId::new(), "east", PortSide::East));
    layout_node.add_port(LayoutPort::new(PortId::new(), "south", PortSide::South));
    layout_node.add_port(LayoutPort::new(PortId::new(), "west", PortSide::West));

    graph.add_node(layout_node);
    node_id_map.insert(node.id.clone(), node_id);

    Ok(node_id)
}

fn convert_edge(
    graph: &mut LayoutGraph,
    node_id_map: &HashMap<String, NodeId>,
    edge: &Edge,
) -> Result<(), String> {
    use tecton_types::edge::{Edge as EdgeType, EdgeId, EdgeKind};

    let from_id = node_id_map
        .get(&edge.from)
        .ok_or_else(|| format!("Source node '{}' not found", edge.from))?;

    for target in &edge.to {
        let to_id = node_id_map
            .get(&target.node_id)
            .ok_or_else(|| format!("Target node '{}' not found", target.node_id))?;

        // Get ports from nodes
        let from_node = graph.get_node(*from_id).ok_or("Source node not in graph")?;
        let to_node = graph.get_node(*to_id).ok_or("Target node not in graph")?;

        // Select appropriate ports based on relative positions
        // For now, assume horizontal layout: source on left, target on right
        let source_port =
            find_port_on_side(from_node, PortSide::East).ok_or("Source node has no east port")?;
        let target_port =
            find_port_on_side(to_node, PortSide::West).ok_or("Target node has no west port")?;

        let edge_kind = match target.op {
            EdgeOp::Arrow => EdgeKind::Default,
            EdgeOp::ThickArrow => EdgeKind::Bold,
            EdgeOp::DashedArrow => EdgeKind::Dashed,
        };

        let mut edge_type =
            EdgeType::new(EdgeId::new(), source_port, target_port).with_kind(edge_kind);

        // Only set label if it's not empty
        if let Some(ref label) = edge.label {
            if !label.is_empty() {
                edge_type = edge_type.with_label(label.clone());
            }
        }

        graph.add_edge(edge_type)?;
    }

    Ok(())
}

/// Find a port on the specified side of a node
fn find_port_on_side(node: &LayoutNode, side: PortSide) -> Option<PortId> {
    node.ports
        .iter()
        .find(|(_, port)| port.position.side == side)
        .map(|(id, _)| *id)
}

fn convert_properties_to_style(properties: &HashMap<String, Value>) -> Result<Style, String> {
    use ordered_float::OrderedFloat;

    let mut style = Style::default();

    for (key, value) in properties {
        match key.as_str() {
            "fill" | "background" => {
                if let Some(color_str) = value.as_string() {
                    style.background = parse_color(color_str)?;
                }
            }
            "stroke" | "border-color" => {
                if let Some(color_str) = value.as_string() {
                    style.border_color = parse_color(color_str)?;
                }
            }
            "stroke-width" | "border-width" => {
                if let Some(n) = value.as_number() {
                    style.border_width = OrderedFloat(n);
                }
            }
            "padding" => {
                if let Some(n) = value.as_number() {
                    style.padding = Padding::uniform(n);
                }
            }
            "margin" => {
                if let Some(n) = value.as_number() {
                    style.margin = Margin::uniform(n);
                }
            }
            "border-radius" => {
                if let Some(n) = value.as_number() {
                    style.border_radius = OrderedFloat(n);
                }
            }
            _ => {}
        }
    }

    Ok(style)
}

fn parse_color(s: &str) -> Result<Color, String> {
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).map_err(|_| "Invalid red component")?;
        let g = u8::from_str_radix(&s[3..5], 16).map_err(|_| "Invalid green component")?;
        let b = u8::from_str_radix(&s[5..7], 16).map_err(|_| "Invalid blue component")?;
        Ok(Color::new(r, g, b))
    } else {
        match s.to_lowercase().as_str() {
            "black" => Ok(Color::BLACK),
            "white" => Ok(Color::WHITE),
            "red" => Ok(Color::RED),
            "green" => Ok(Color::GREEN),
            "blue" => Ok(Color::BLUE),
            "gray" | "grey" => Ok(Color::GRAY_500),
            "transparent" => Ok(Color::TRANSPARENT),
            _ => Err(format!("Unknown color: {}", s)),
        }
    }
}

fn parse_shape_type(s: &str) -> ShapeType {
    match s {
        "Rectangle" => ShapeType::Rectangle,
        "Circle" => ShapeType::Circle,
        "Diamond" => ShapeType::Diamond,
        "Cylinder" => ShapeType::Cylinder,
        "Parallelogram" => ShapeType::Parallelogram,
        "Hexagon" => ShapeType::Hexagon,
        "Cloud" => ShapeType::Cloud,
        _ => ShapeType::Rectangle,
    }
}

fn merge_styles(base: Style, override_style: Style) -> Style {
    use ordered_float::OrderedFloat;
    Style {
        background: if override_style.background != Color::WHITE {
            override_style.background
        } else {
            base.background
        },
        border_color: if override_style.border_color != Color::GRAY_300 {
            override_style.border_color
        } else {
            base.border_color
        },
        border_width: if override_style.border_width != OrderedFloat(1.0) {
            override_style.border_width
        } else {
            base.border_width
        },
        border_radius: if override_style.border_radius != OrderedFloat(0.0) {
            override_style.border_radius
        } else {
            base.border_radius
        },
        shape: if override_style.shape != ShapeType::Rectangle {
            override_style.shape
        } else {
            base.shape
        },
        ..override_style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_node() {
        let dsl = r##"
            MyNode [label: "Test Node", fill: "#ff0000"]
        "##;

        let file = tecton_dsl::parse_dsl(dsl).unwrap();
        let graph = convert_to_layout_graph(&file).unwrap();

        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_convert_container_with_children() {
        let dsl = r##"
            Row {
                Node1 [label: "A"]
                Node2 [label: "B"]
            }
        "##;

        let file = tecton_dsl::parse_dsl(dsl).unwrap();
        let graph = convert_to_layout_graph(&file).unwrap();

        assert_eq!(graph.node_count(), 3);
    }

    #[test]
    fn test_convert_with_edges() {
        let dsl = r##"
            A [label: "Node A"]
            B [label: "Node B"]
            A -> B : "connects to"
        "##;

        let file = tecton_dsl::parse_dsl(dsl).unwrap();
        let graph = convert_to_layout_graph(&file).unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("#ff0000").unwrap(), Color::RED);
        assert_eq!(parse_color("black").unwrap(), Color::BLACK);
    }
}
