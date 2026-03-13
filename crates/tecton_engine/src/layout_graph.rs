use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use tecton_types::{
    edge::Edge,
    geometry::{Point, Rect},
    id::{EdgeId, NodeId, PortId, PortRef},
    shape::{ShapeFactory, ShapeGeometry, ShapeType},
    style::Style,
};

/// A node in the layout graph
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    pub id: NodeId,
    pub label: String,
    pub style: Style,
    pub shape: ShapeType,
    pub ports: HashMap<PortId, LayoutPort>,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub bounds: Option<Rect>,
}

impl LayoutNode {
    pub fn new(id: NodeId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            style: Style::default(),
            shape: ShapeType::Rectangle,
            ports: HashMap::new(),
            parent: None,
            children: Vec::new(),
            bounds: None,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_shape(mut self, shape: ShapeType) -> Self {
        self.shape = shape;
        self
    }

    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn add_port(&mut self, port: LayoutPort) {
        self.ports.insert(port.id, port);
    }

    pub fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }

    pub fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = Some(bounds);
    }

    pub fn get_port_position(&self, port_id: PortId) -> Option<Point> {
        let port = self.ports.get(&port_id)?;
        let bounds = self.bounds?;
        Some(port.position.absolute_position(&bounds))
    }
}

/// A port on a node
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutPort {
    pub id: PortId,
    pub name: String,
    pub position: tecton_types::geometry::Port,
}

impl LayoutPort {
    pub fn new(
        id: PortId,
        name: impl Into<String>,
        side: tecton_types::geometry::PortSide,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            position: tecton_types::geometry::Port::new(side, 0.5),
        }
    }

    pub fn with_position(mut self, position: f32) -> Self {
        self.position = tecton_types::geometry::Port::new(self.position.side, position);
        self
    }
}

/// The layout graph containing nodes, edges, and their relationships
#[derive(Debug, Clone)]
pub struct LayoutGraph {
    /// The underlying petgraph
    graph: DiGraph<LayoutNode, Edge>,
    /// Map from NodeId to graph index
    node_indices: HashMap<NodeId, NodeIndex>,
    /// Map from EdgeId to (source, target) node indices
    edge_map: HashMap<EdgeId, (NodeIndex, NodeIndex)>,
    /// Root node ID
    root: Option<NodeId>,
}

impl Default for LayoutGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            edge_map: HashMap::new(),
            root: None,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: LayoutNode) -> NodeId {
        let id = node.id;
        let idx = self.graph.add_node(node);
        self.node_indices.insert(id, idx);

        if self.root.is_none() {
            self.root = Some(id);
        }

        id
    }

    /// Add an edge between two nodes
    pub fn add_edge(&mut self, edge: Edge) -> Result<(), String> {
        // Find nodes that have these ports
        let source_node = self.find_node_by_port(edge.source_port)?;
        let target_node = self.find_node_by_port(edge.target_port)?;

        let source_idx = *self
            .node_indices
            .get(&source_node)
            .ok_or_else(|| "Source node not found".to_string())?;
        let target_idx = *self
            .node_indices
            .get(&target_node)
            .ok_or_else(|| "Target node not found".to_string())?;

        let edge_id = edge.id;
        self.graph.add_edge(source_idx, target_idx, edge);
        self.edge_map.insert(edge_id, (source_idx, target_idx));

        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, id: NodeId) -> Option<&LayoutNode> {
        self.node_indices
            .get(&id)
            .and_then(|idx| self.graph.node_weight(*idx))
    }

    /// Get a node by ID (mutable)
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut LayoutNode> {
        self.node_indices
            .get(&id)
            .and_then(|idx| self.graph.node_weight_mut(*idx))
    }

    /// Get an edge by ID
    pub fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edge_map.get(&id).and_then(|(source_idx, target_idx)| {
            self.graph
                .edges_connecting(*source_idx, *target_idx)
                .find(|e| e.weight().id == id)
                .map(|e| e.weight())
        })
    }

    /// Find the node that owns a given port
    pub fn find_node_by_port(&self, port_id: PortId) -> Result<NodeId, String> {
        for (node_id, idx) in &self.node_indices {
            if let Some(node) = self.graph.node_weight(*idx) {
                if node.ports.contains_key(&port_id) {
                    return Ok(*node_id);
                }
            }
        }
        Err(format!("Port not found: {:?}", port_id))
    }

    /// Get all nodes
    pub fn nodes(&self) -> Vec<&LayoutNode> {
        self.graph.node_weights().collect()
    }

    /// Get all edges
    pub fn edges(&self) -> Vec<&Edge> {
        self.graph.edge_weights().collect()
    }

    /// Get the root node
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Set the root node
    pub fn set_root(&mut self, root: NodeId) {
        self.root = Some(root);
    }

    /// Get children of a node
    pub fn children(&self, parent_id: NodeId) -> Vec<&LayoutNode> {
        let mut result = Vec::new();

        if let Some(parent) = self.get_node(parent_id) {
            for child_id in &parent.children {
                if let Some(child) = self.get_node(*child_id) {
                    result.push(child);
                }
            }
        }

        result
    }

    /// Build a map of node bounds
    pub fn node_bounds(&self) -> HashMap<NodeId, Rect> {
        let mut bounds = HashMap::new();

        for node in self.nodes() {
            if let Some(rect) = node.bounds {
                bounds.insert(node.id, rect);
            }
        }

        bounds
    }

    /// Build a map of port positions
    pub fn port_positions(&self) -> HashMap<PortId, Point> {
        let mut positions = HashMap::new();

        for node in self.nodes() {
            for (port_id, port) in &node.ports {
                if let Some(bounds) = node.bounds {
                    let pos = port.position.absolute_position(&bounds);
                    positions.insert(*port_id, pos);
                }
            }
        }

        positions
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_graph() {
        let mut graph = LayoutGraph::new();

        let node1 = LayoutNode::new(NodeId::new(), "Node 1");
        let node2 = LayoutNode::new(NodeId::new(), "Node 2");

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);

        assert_eq!(graph.node_count(), 2);

        let port1 = PortId::new();
        let port2 = PortId::new();

        let edge = Edge::new(EdgeId::new(), port1, port2);

        // Add ports to nodes first
        graph.get_node_mut(id1).unwrap().add_port(LayoutPort::new(
            port1,
            "out",
            tecton_types::geometry::PortSide::East,
        ));
        graph.get_node_mut(id2).unwrap().add_port(LayoutPort::new(
            port2,
            "in",
            tecton_types::geometry::PortSide::West,
        ));

        graph.add_edge(edge).unwrap();

        assert_eq!(graph.edge_count(), 1);
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_layout_node_new() {
        let id = NodeId::new();
        let node = LayoutNode::new(id, "Test");

        assert_eq!(node.id, id);
        assert_eq!(node.label, "Test");
        assert!(node.ports.is_empty());
        assert!(node.children.is_empty());
        assert!(node.parent.is_none());
        assert!(node.bounds.is_none());
    }

    #[test]
    fn test_layout_node_with_style() {
        let style = Style::default();
        let node = LayoutNode::new(NodeId::new(), "Test").with_style(style.clone());

        assert_eq!(node.style, style);
    }

    #[test]
    fn test_layout_node_with_shape() {
        let node = LayoutNode::new(NodeId::new(), "Test").with_shape(ShapeType::Circle);

        assert_eq!(node.shape, ShapeType::Circle);
    }

    #[test]
    fn test_layout_node_with_parent() {
        let parent_id = NodeId::new();
        let node = LayoutNode::new(NodeId::new(), "Child").with_parent(parent_id);

        assert_eq!(node.parent, Some(parent_id));
    }

    #[test]
    fn test_layout_node_add_port() {
        let mut node = LayoutNode::new(NodeId::new(), "Test");
        let port = LayoutPort::new(PortId::new(), "in", tecton_types::geometry::PortSide::West);

        node.add_port(port);

        assert_eq!(node.ports.len(), 1);
    }

    #[test]
    fn test_layout_node_add_child() {
        let mut node = LayoutNode::new(NodeId::new(), "Parent");
        let child_id = NodeId::new();

        node.add_child(child_id);

        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0], child_id);
    }

    #[test]
    fn test_layout_node_set_bounds() {
        let mut node = LayoutNode::new(NodeId::new(), "Test");
        let bounds = Rect::new(10.0, 20.0, 100.0, 50.0);

        node.set_bounds(bounds);

        assert_eq!(node.bounds, Some(bounds));
    }

    #[test]
    fn test_layout_port_new() {
        let id = PortId::new();
        let port = LayoutPort::new(id, "api", tecton_types::geometry::PortSide::East);

        assert_eq!(port.id, id);
        assert_eq!(port.name, "api");
        assert_eq!(port.position.side, tecton_types::geometry::PortSide::East);
    }

    #[test]
    fn test_layout_port_with_position() {
        let port = LayoutPort::new(
            PortId::new(),
            "test",
            tecton_types::geometry::PortSide::North,
        )
        .with_position(0.75);

        assert!((port.position.position.into_inner() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_layout_graph_default() {
        let graph = LayoutGraph::default();

        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_layout_graph_add_node() {
        let mut graph = LayoutGraph::new();

        let node = LayoutNode::new(NodeId::new(), "Test");
        let id = graph.add_node(node);

        assert_eq!(graph.node_count(), 1);
        assert!(graph.get_node(id).is_some());
    }

    #[test]
    fn test_layout_graph_get_node_mut() {
        let mut graph = LayoutGraph::new();

        let id = NodeId::new();
        let node = LayoutNode::new(id, "Test");
        graph.add_node(node);

        let node_ref = graph.get_node_mut(id);
        assert!(node_ref.is_some());

        node_ref.unwrap().label = "Modified".to_string();

        assert_eq!(graph.get_node(id).unwrap().label, "Modified");
    }

    #[test]
    fn test_layout_graph_nodes() {
        let mut graph = LayoutGraph::new();

        graph.add_node(LayoutNode::new(NodeId::new(), "A"));
        graph.add_node(LayoutNode::new(NodeId::new(), "B"));

        let nodes = graph.nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_layout_graph_edges() {
        let mut graph = LayoutGraph::new();

        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let port1 = PortId::new();
        let port2 = PortId::new();

        let mut node1 = LayoutNode::new(id1, "A");
        node1.add_port(LayoutPort::new(
            port1,
            "out",
            tecton_types::geometry::PortSide::East,
        ));

        let mut node2 = LayoutNode::new(id2, "B");
        node2.add_port(LayoutPort::new(
            port2,
            "in",
            tecton_types::geometry::PortSide::West,
        ));

        graph.add_node(node1);
        graph.add_node(node2);

        let edge = Edge::new(EdgeId::new(), port1, port2);
        graph.add_edge(edge).unwrap();

        let edges = graph.edges();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_layout_graph_root() {
        let mut graph = LayoutGraph::new();

        let id = NodeId::new();
        graph.add_node(LayoutNode::new(id, "Root"));

        assert_eq!(graph.root(), Some(id));
    }

    #[test]
    fn test_layout_graph_set_root() {
        let mut graph = LayoutGraph::new();
        let id = NodeId::new();

        graph.set_root(id);

        assert_eq!(graph.root(), Some(id));
    }

    #[test]
    fn test_layout_graph_children() {
        let mut graph = LayoutGraph::new();

        let parent_id = NodeId::new();
        let child_id = NodeId::new();

        let mut parent = LayoutNode::new(parent_id, "Parent");
        parent.children.push(child_id);

        graph.add_node(parent);
        graph.add_node(LayoutNode::new(child_id, "Child"));

        let children = graph.children(parent_id);
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn test_layout_graph_node_bounds() {
        let mut graph = LayoutGraph::new();

        let id = NodeId::new();
        let mut node = LayoutNode::new(id, "Test");
        node.set_bounds(Rect::new(0.0, 0.0, 100.0, 50.0));

        graph.add_node(node);

        let bounds = graph.node_bounds();
        assert_eq!(bounds.len(), 1);
        assert!(bounds.contains_key(&id));
    }

    #[test]
    fn test_layout_graph_port_positions() {
        let mut graph = LayoutGraph::new();

        let id = NodeId::new();
        let port_id = PortId::new();
        let mut node = LayoutNode::new(id, "Test");
        node.add_port(LayoutPort::new(
            port_id,
            "test",
            tecton_types::geometry::PortSide::East,
        ));
        node.set_bounds(Rect::new(0.0, 0.0, 100.0, 50.0));

        graph.add_node(node);

        let positions = graph.port_positions();
        assert_eq!(positions.len(), 1);
        assert!(positions.contains_key(&port_id));
    }

    #[test]
    fn test_layout_graph_find_node_by_port() {
        let mut graph = LayoutGraph::new();

        let node_id = NodeId::new();
        let port_id = PortId::new();

        let mut node = LayoutNode::new(node_id, "Test");
        node.add_port(LayoutPort::new(
            port_id,
            "test",
            tecton_types::geometry::PortSide::East,
        ));

        graph.add_node(node);

        let found = graph.find_node_by_port(port_id);
        assert!(found.is_ok());
        assert_eq!(found.unwrap(), node_id);
    }

    #[test]
    fn test_layout_graph_find_node_by_port_not_found() {
        let graph = LayoutGraph::new();
        let result = graph.find_node_by_port(PortId::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_layout_graph_add_edge_missing_port() {
        let mut graph = LayoutGraph::new();

        // Add nodes without the ports
        graph.add_node(LayoutNode::new(NodeId::new(), "A"));
        graph.add_node(LayoutNode::new(NodeId::new(), "B"));

        let edge = Edge::new(EdgeId::new(), PortId::new(), PortId::new());
        let result = graph.add_edge(edge);

        assert!(result.is_err());
    }
}
