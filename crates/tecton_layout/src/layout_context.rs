use std::collections::HashMap;

use tecton_types::{geometry::Rect, id::NodeId, style::Style};

/// Visual spacing of a row of the classic automatic SVG card label.
pub const NODE_LABEL_LINE_HEIGHT: f32 = 17.0;

/// Split newline-delimited card labels without dropping intentional blank rows.
pub fn node_label_lines(label: &str) -> impl Iterator<Item = &str> {
    label
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
}

/// Estimate the classic card size from the widest row and number of rows.
/// The original single-row estimate remains unchanged.
pub fn estimate_leaf_label_size(label: &str) -> (f32, f32) {
    if !label.contains('\n') {
        return (label.len() as f32 * 10.0 + 40.0, 50.0);
    }
    let (longest, count) = node_label_lines(label).fold((0, 0), |(width, count), line| {
        (width.max(line.len()), count + 1)
    });
    (
        longest as f32 * 10.0 + 40.0,
        50.0 + (count - 1) as f32 * NODE_LABEL_LINE_HEIGHT,
    )
}

/// Result of a layout computation for a single node
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    /// The node ID
    pub id: NodeId,
    /// The computed bounding rectangle
    pub rect: Rect,
    /// The effective style (after inheritance)
    pub style: Style,
    /// The node label for rendering
    pub label: String,
    /// Child nodes (if this is a container)
    pub children: Vec<LayoutNode>,
    /// Z-index for layering
    pub z_index: i32,
}

impl LayoutNode {
    pub fn new(id: NodeId, rect: Rect, style: Style) -> Self {
        Self {
            id,
            rect,
            style,
            label: String::new(),
            children: Vec::new(),
            z_index: 0,
        }
    }

    pub fn with_children(mut self, children: Vec<LayoutNode>) -> Self {
        self.children = children;
        self
    }

    pub fn with_z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Get the content rectangle (inside padding)
    pub fn content_rect(&self) -> Rect {
        self.rect.inset(
            self.style.padding.left() + self.style.border_width(),
            self.style.padding.top() + self.style.border_width(),
        )
    }

    /// Get the inner size (available for children)
    pub fn inner_size(&self) -> tecton_types::geometry::Size {
        let content = self.content_rect();
        tecton_types::geometry::Size::new(content.width(), content.height())
    }

    /// Find a node by ID recursively
    pub fn find_node(&self, id: NodeId) -> Option<&LayoutNode> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find_node(id))
    }

    /// Find a node by ID recursively (mutable)
    pub fn find_node_mut(&mut self, id: NodeId) -> Option<&mut LayoutNode> {
        if self.id == id {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_node_mut(id))
    }

    /// Collect all nodes into a flat map
    pub fn collect_nodes<'a>(&'a self, map: &mut HashMap<NodeId, &'a LayoutNode>) {
        map.insert(self.id, self);
        for child in &self.children {
            child.collect_nodes(map);
        }
    }
}

/// Complete layout result containing all positioned nodes
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutResult {
    /// Root node of the layout tree
    pub root: LayoutNode,
    /// Canvas bounds
    pub bounds: Rect,
}

impl LayoutResult {
    pub fn new(root: LayoutNode, bounds: Rect) -> Self {
        Self { root, bounds }
    }

    /// Find a node by ID
    pub fn find_node(&self, id: NodeId) -> Option<&LayoutNode> {
        self.root.find_node(id)
    }

    /// Get all nodes as a flat map
    pub fn node_map(&self) -> HashMap<NodeId, &LayoutNode> {
        let mut map = HashMap::new();
        self.root.collect_nodes(&mut map);
        map
    }

    /// Get the bounding box of all nodes
    pub fn bounding_box(&self) -> Rect {
        let mut nodes = Vec::new();
        self.collect_rects(&self.root, &mut nodes);

        if nodes.is_empty() {
            return Rect::default();
        }

        nodes.iter().fold(nodes[0], |acc, rect| acc.union(rect))
    }

    fn collect_rects(&self, node: &LayoutNode, rects: &mut Vec<Rect>) {
        rects.push(node.rect);
        for child in &node.children {
            self.collect_rects(child, rects);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tecton_types::geometry::{Point, Size};

    #[test]
    fn test_layout_node_content_rect() {
        let style = Style {
            padding: tecton_types::style::Padding::uniform(10.0),
            border_width: 2.0.into(),
            ..Default::default()
        };
        let node = LayoutNode::new(NodeId::new(), Rect::new(0.0, 0.0, 100.0, 100.0), style);

        let content = node.content_rect();
        assert_eq!(content.x(), 12.0); // padding.left + border_width
        assert_eq!(content.y(), 12.0);
        assert_eq!(content.width(), 76.0); // 100 - 2*12
        assert_eq!(content.height(), 76.0);
    }

    #[test]
    fn test_find_node() {
        let child1 = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 50.0, 50.0),
            Style::default(),
        );
        let child2 = LayoutNode::new(
            NodeId::new(),
            Rect::new(50.0, 0.0, 50.0, 50.0),
            Style::default(),
        );
        let child2_id = child2.id;

        let root = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Style::default(),
        )
        .with_children(vec![child1, child2]);

        let result = LayoutResult::new(root, Rect::new(0.0, 0.0, 100.0, 100.0));

        assert!(result.find_node(child2_id).is_some());
        assert!(result.find_node(NodeId::new()).is_none());
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_layout_node_new() {
        let id = NodeId::new();
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let style = Style::default();

        let node = LayoutNode::new(id, rect, style.clone());

        assert_eq!(node.id, id);
        assert_eq!(node.rect, rect);
        assert_eq!(node.style, style);
        assert!(node.label.is_empty());
        assert!(node.children.is_empty());
        assert_eq!(node.z_index, 0);
    }

    #[test]
    fn test_layout_node_with_label() {
        let node = LayoutNode::new(NodeId::new(), Rect::default(), Style::default())
            .with_label("Test Label");

        assert_eq!(node.label, "Test Label");
    }

    #[test]
    fn test_layout_node_with_z_index() {
        let node =
            LayoutNode::new(NodeId::new(), Rect::default(), Style::default()).with_z_index(10);

        assert_eq!(node.z_index, 10);
    }

    #[test]
    fn test_layout_node_inner_size() {
        let style = Style {
            padding: tecton_types::style::Padding::uniform(10.0),
            border_width: 5.0.into(),
            ..Default::default()
        };
        let node = LayoutNode::new(NodeId::new(), Rect::new(0.0, 0.0, 100.0, 100.0), style);

        let inner = node.inner_size();
        // content_rect: x=15, y=15, width=70, height=70
        assert_eq!(inner.width(), 70.0);
        assert_eq!(inner.height(), 70.0);
    }

    #[test]
    fn test_layout_node_find_node_self() {
        let id = NodeId::new();
        let node = LayoutNode::new(id, Rect::default(), Style::default());

        let found = node.find_node(id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[test]
    fn test_layout_node_find_node_deep_child() {
        let grandchild = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 25.0, 25.0),
            Style::default(),
        );
        let grandchild_id = grandchild.id;

        let child = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 50.0, 50.0),
            Style::default(),
        )
        .with_children(vec![grandchild]);

        let root = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Style::default(),
        )
        .with_children(vec![child]);

        let found = root.find_node(grandchild_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, grandchild_id);
    }

    #[test]
    fn test_layout_node_find_node_mut() {
        let id = NodeId::new();
        let mut node = LayoutNode::new(id, Rect::new(0.0, 0.0, 100.0, 100.0), Style::default());

        let found = node.find_node_mut(id);
        assert!(found.is_some());
        found.unwrap().z_index = 5;

        assert_eq!(node.z_index, 5);
    }

    #[test]
    fn test_layout_node_find_node_not_found() {
        let node = LayoutNode::new(NodeId::new(), Rect::default(), Style::default());

        let found = node.find_node(NodeId::new());
        assert!(found.is_none());
    }

    #[test]
    fn test_layout_node_collect_nodes() {
        let child1 = LayoutNode::new(NodeId::new(), Rect::default(), Style::default());
        let child2 = LayoutNode::new(NodeId::new(), Rect::default(), Style::default());
        let child1_id = child1.id;
        let child2_id = child2.id;

        let root = LayoutNode::new(NodeId::new(), Rect::default(), Style::default())
            .with_children(vec![child1, child2]);
        let root_id = root.id;

        let mut map = HashMap::new();
        root.collect_nodes(&mut map);

        assert_eq!(map.len(), 3);
        assert!(map.contains_key(&root_id));
        assert!(map.contains_key(&child1_id));
        assert!(map.contains_key(&child2_id));
    }

    #[test]
    fn test_layout_result_new() {
        let root = LayoutNode::new(NodeId::new(), Rect::default(), Style::default());
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);

        let result = LayoutResult::new(root.clone(), bounds);

        assert_eq!(result.root, root);
        assert_eq!(result.bounds, bounds);
    }

    #[test]
    fn test_layout_result_node_map() {
        let child1 = LayoutNode::new(NodeId::new(), Rect::default(), Style::default());
        let child2 = LayoutNode::new(NodeId::new(), Rect::default(), Style::default());

        let root = LayoutNode::new(NodeId::new(), Rect::default(), Style::default())
            .with_children(vec![child1, child2]);

        let result = LayoutResult::new(root, Rect::default());

        let map = result.node_map();
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_layout_result_bounding_box_single() {
        let root = LayoutNode::new(
            NodeId::new(),
            Rect::new(10.0, 10.0, 100.0, 50.0),
            Style::default(),
        );

        let result = LayoutResult::new(root, Rect::new(0.0, 0.0, 800.0, 600.0));

        let bbox = result.bounding_box();
        assert_eq!(bbox.x(), 10.0);
        assert_eq!(bbox.y(), 10.0);
        assert_eq!(bbox.width(), 100.0);
        assert_eq!(bbox.height(), 50.0);
    }

    #[test]
    fn test_layout_result_bounding_box_multiple() {
        let child1 = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 50.0, 50.0),
            Style::default(),
        );
        let child2 = LayoutNode::new(
            NodeId::new(),
            Rect::new(100.0, 100.0, 50.0, 50.0),
            Style::default(),
        );

        let root = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 0.0, 0.0),
            Style::default(),
        )
        .with_children(vec![child1, child2]);

        let result = LayoutResult::new(root, Rect::default());

        let bbox = result.bounding_box();
        // Bounding box should contain both children
        assert_eq!(bbox.x(), 0.0);
        assert_eq!(bbox.y(), 0.0);
        assert_eq!(bbox.right(), 150.0);
        assert_eq!(bbox.bottom(), 150.0);
    }

    #[test]
    fn test_layout_result_bounding_box_empty() {
        let root = LayoutNode::new(NodeId::new(), Rect::default(), Style::default());
        let result = LayoutResult::new(root, Rect::default());

        let bbox = result.bounding_box();
        assert_eq!(bbox, Rect::default());
    }

    #[test]
    fn test_layout_result_empty_result() {
        let root = LayoutNode::new(NodeId::new(), Rect::default(), Style::default());
        let result = LayoutResult::new(root, Rect::default());

        let found = result.find_node(NodeId::new());
        assert!(found.is_none());
    }

    #[test]
    fn test_layout_result_multiple_roots_scenario() {
        // Simulate multiple "roots" via children
        let child1 = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Style::default(),
        );
        let child2 = LayoutNode::new(
            NodeId::new(),
            Rect::new(100.0, 0.0, 100.0, 100.0),
            Style::default(),
        );

        let root = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 200.0, 100.0),
            Style::default(),
        )
        .with_children(vec![child1, child2]);

        let result = LayoutResult::new(root, Rect::new(0.0, 0.0, 200.0, 100.0));

        let map = result.node_map();
        assert_eq!(map.len(), 3); // root + 2 children
    }
}
