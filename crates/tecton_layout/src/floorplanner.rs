use tecton_types::{
    geometry::{Rect, Size},
    id::NodeId,
    mass::VisualMass,
};

use crate::mass_calculator::MassCalculator;

/// A region in the floorplan
#[derive(Debug, Clone, PartialEq)]
pub struct FloorplanRegion {
    /// The bounding rectangle
    pub rect: Rect,
    /// The node assigned to this region (if any)
    pub node_id: Option<NodeId>,
    /// Child regions
    pub children: Vec<FloorplanRegion>,
    /// The split direction
    pub split_direction: Option<SplitDirection>,
}

impl FloorplanRegion {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            node_id: None,
            children: Vec::new(),
            split_direction: None,
        }
    }

    pub fn with_node(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    pub fn with_children(mut self, children: Vec<FloorplanRegion>) -> Self {
        self.children = children;
        self
    }

    pub fn with_split_direction(mut self, direction: SplitDirection) -> Self {
        self.split_direction = Some(direction);
        self
    }

    /// Get the area of this region
    pub fn area(&self) -> f32 {
        self.rect.width() * self.rect.height()
    }

    /// Get the aspect ratio (width / height)
    pub fn aspect_ratio(&self) -> f32 {
        if self.rect.height() == 0.0 {
            return f32::INFINITY;
        }
        self.rect.width() / self.rect.height()
    }

    /// Check if this region is a leaf (has no children)
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// Direction for splitting a region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SplitDirection {
    Horizontal, // Split along X axis (creates left/right)
    Vertical,   // Split along Y axis (creates top/bottom)
}

impl SplitDirection {
    /// Choose the best split direction based on the region's aspect ratio
    pub fn choose_for_aspect_ratio(aspect_ratio: f32) -> Self {
        // If the region is wider than tall, split horizontally (left/right)
        // If the region is taller than wide, split vertically (top/bottom)
        if aspect_ratio > 1.0 {
            SplitDirection::Horizontal
        } else {
            SplitDirection::Vertical
        }
    }

    /// Get the opposite direction
    pub fn opposite(&self) -> Self {
        match self {
            SplitDirection::Horizontal => SplitDirection::Vertical,
            SplitDirection::Vertical => SplitDirection::Horizontal,
        }
    }
}

/// Floorplanner using weighted recursive bisection
///
/// This algorithm recursively divides the canvas into regions based on
/// the visual mass of nodes, aiming to produce a balanced, square-like layout.
#[derive(Debug, Clone)]
pub struct BisectionFloorplanner {
    calculator: MassCalculator,
    /// Minimum region size (to prevent infinite recursion)
    min_region_size: f32,
    /// Target aspect ratio (1.0 = square)
    target_aspect_ratio: f32,
}

impl Default for BisectionFloorplanner {
    fn default() -> Self {
        Self {
            calculator: MassCalculator::new(),
            min_region_size: 20.0,
            target_aspect_ratio: 1.0,
        }
    }
}

impl BisectionFloorplanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_min_region_size(mut self, size: f32) -> Self {
        self.min_region_size = size;
        self
    }

    pub fn with_target_aspect_ratio(mut self, ratio: f32) -> Self {
        self.target_aspect_ratio = ratio;
        self
    }

    /// Plan the layout for a set of nodes
    ///
    /// # Arguments
    /// * `bounds` - The available canvas area
    /// * `nodes` - List of (node_id, mass) pairs
    pub fn plan(&mut self, bounds: Rect, nodes: Vec<(NodeId, VisualMass)>) -> FloorplanRegion {
        if nodes.is_empty() {
            return FloorplanRegion::new(bounds);
        }

        // Cache the masses
        for (id, mass) in &nodes {
            self.calculator.insert_mass(*id, *mass);
        }

        self.bisect_region(bounds, nodes)
    }

    fn bisect_region(&self, bounds: Rect, nodes: Vec<(NodeId, VisualMass)>) -> FloorplanRegion {
        // Base case: single node
        if nodes.len() == 1 {
            return FloorplanRegion::new(bounds).with_node(nodes[0].0);
        }

        // Base case: region too small
        if bounds.width() < self.min_region_size || bounds.height() < self.min_region_size {
            // Just assign all nodes to this region (they'll be handled by Taffy)
            return FloorplanRegion::new(bounds);
        }

        // Choose split direction based on aspect ratio
        let aspect_ratio = bounds.width() / bounds.height();
        let split_direction = SplitDirection::choose_for_aspect_ratio(aspect_ratio);

        // Find the split point based on mass
        let split_point = self.find_mass_split_point(&nodes);

        // Split the region
        let (left_nodes, right_nodes) = nodes.split_at(split_point);

        let left_mass: f32 = left_nodes.iter().map(|(_, m)| m.total()).sum();
        let right_mass: f32 = right_nodes.iter().map(|(_, m)| m.total()).sum();
        let total_mass = left_mass + right_mass;

        let left_ratio = if total_mass > 0.0 {
            left_mass / total_mass
        } else {
            0.5
        };

        let (left_rect, right_rect) = match split_direction {
            SplitDirection::Horizontal => {
                let split_x = (bounds.x() + bounds.width() * left_ratio).round();
                (
                    Rect::new(
                        bounds.x(),
                        bounds.y(),
                        split_x - bounds.x(),
                        bounds.height(),
                    ),
                    Rect::new(
                        split_x,
                        bounds.y(),
                        bounds.right() - split_x,
                        bounds.height(),
                    ),
                )
            }
            SplitDirection::Vertical => {
                let split_y = (bounds.y() + bounds.height() * left_ratio).round();
                (
                    Rect::new(bounds.x(), bounds.y(), bounds.width(), split_y - bounds.y()),
                    Rect::new(
                        bounds.x(),
                        split_y,
                        bounds.width(),
                        bounds.bottom() - split_y,
                    ),
                )
            }
        };

        // Recursively bisect
        let left_region = self.bisect_region(left_rect, left_nodes.to_vec());
        let right_region = self.bisect_region(right_rect, right_nodes.to_vec());

        FloorplanRegion::new(bounds)
            .with_split_direction(split_direction)
            .with_children(vec![left_region, right_region])
    }

    fn find_mass_split_point(&self, nodes: &[(NodeId, VisualMass)]) -> usize {
        if nodes.len() < 2 {
            return nodes.len();
        }

        let total_mass: f32 = nodes.iter().map(|(_, m)| m.total()).sum();
        let target = total_mass / 2.0;

        let mut cumulative = 0.0;
        for (i, (_, mass)) in nodes.iter().enumerate().take(nodes.len() - 1) {
            cumulative += mass.total();
            if cumulative >= target {
                // Try to keep the split balanced
                let prev_diff = (cumulative - mass.total() - target).abs();
                let curr_diff = (cumulative - target).abs();

                if prev_diff < curr_diff && i > 0 {
                    return i;
                }
                return i + 1;
            }
        }

        nodes.len() / 2
    }

    /// Get the mass calculator
    pub fn calculator(&self) -> &MassCalculator {
        &self.calculator
    }
}

/// A simpler floorplanner that just divides space equally
#[derive(Debug, Clone)]
pub struct GridFloorplanner {
    cols: usize,
    rows: usize,
}

impl GridFloorplanner {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }

    pub fn plan(&self, bounds: Rect, nodes: Vec<NodeId>) -> FloorplanRegion {
        let cell_width = bounds.width() / self.cols as f32;
        let cell_height = bounds.height() / self.rows as f32;

        let mut children = Vec::new();

        for (i, node_id) in nodes.iter().enumerate() {
            let col = i % self.cols;
            let row = i / self.cols;

            if row >= self.rows {
                break;
            }

            let x = bounds.x() + col as f32 * cell_width;
            let y = bounds.y() + row as f32 * cell_height;

            let rect = Rect::new(x, y, cell_width, cell_height);
            children.push(FloorplanRegion::new(rect).with_node(*node_id));
        }

        FloorplanRegion::new(bounds).with_children(children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_direction() {
        assert_eq!(
            SplitDirection::choose_for_aspect_ratio(2.0),
            SplitDirection::Horizontal
        );
        assert_eq!(
            SplitDirection::choose_for_aspect_ratio(0.5),
            SplitDirection::Vertical
        );
    }

    #[test]
    fn test_bisection_single_node() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let node_id = NodeId::new();

        let region = planner.plan(bounds, vec![(node_id, VisualMass::new(10.0))]);

        assert_eq!(region.node_id, Some(node_id));
        assert!(region.children.is_empty());
    }

    #[test]
    fn test_bisection_two_nodes() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let node1 = NodeId::new();
        let node2 = NodeId::new();

        let region = planner.plan(
            bounds,
            vec![
                (node1, VisualMass::new(10.0)),
                (node2, VisualMass::new(10.0)),
            ],
        );

        assert_eq!(region.children.len(), 2);
        assert!(
            region.children[0].node_id == Some(node1) || region.children[0].node_id == Some(node2)
        );
    }

    #[test]
    fn test_grid_floorplanner() {
        let planner = GridFloorplanner::new(2, 2);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let nodes: Vec<NodeId> = (0..4).map(|_| NodeId::new()).collect();
        let region = planner.plan(bounds, nodes.clone());

        assert_eq!(region.children.len(), 4);
        assert_eq!(region.children[0].rect.width(), 50.0);
        assert_eq!(region.children[0].rect.height(), 50.0);
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_split_direction_opposite() {
        assert_eq!(
            SplitDirection::Horizontal.opposite(),
            SplitDirection::Vertical
        );
        assert_eq!(
            SplitDirection::Vertical.opposite(),
            SplitDirection::Horizontal
        );
    }

    #[test]
    fn test_split_direction_at_boundary() {
        // At exactly 1.0, choose vertical
        assert_eq!(
            SplitDirection::choose_for_aspect_ratio(1.0),
            SplitDirection::Vertical
        );
    }

    #[test]
    fn test_floorplan_region_new() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let region = FloorplanRegion::new(rect);

        assert_eq!(region.rect, rect);
        assert_eq!(region.node_id, None);
        assert!(region.children.is_empty());
        assert_eq!(region.split_direction, None);
    }

    #[test]
    fn test_floorplan_region_with_node() {
        let node_id = NodeId::new();
        let region = FloorplanRegion::new(Rect::new(0.0, 0.0, 100.0, 100.0)).with_node(node_id);

        assert_eq!(region.node_id, Some(node_id));
    }

    #[test]
    fn test_floorplan_region_with_children() {
        let child1 = FloorplanRegion::new(Rect::new(0.0, 0.0, 50.0, 100.0));
        let child2 = FloorplanRegion::new(Rect::new(50.0, 0.0, 50.0, 100.0));

        let region = FloorplanRegion::new(Rect::new(0.0, 0.0, 100.0, 100.0))
            .with_children(vec![child1, child2]);

        assert_eq!(region.children.len(), 2);
    }

    #[test]
    fn test_floorplan_region_with_split_direction() {
        let region = FloorplanRegion::new(Rect::new(0.0, 0.0, 100.0, 100.0))
            .with_split_direction(SplitDirection::Horizontal);

        assert_eq!(region.split_direction, Some(SplitDirection::Horizontal));
    }

    #[test]
    fn test_floorplan_region_area() {
        let region = FloorplanRegion::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(region.area(), 5000.0);
    }

    #[test]
    fn test_floorplan_region_aspect_ratio() {
        let region = FloorplanRegion::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert!((region.aspect_ratio() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_floorplan_region_is_leaf() {
        let leaf = FloorplanRegion::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(leaf.is_leaf());

        let non_leaf = FloorplanRegion::new(Rect::new(0.0, 0.0, 100.0, 100.0))
            .with_children(vec![FloorplanRegion::new(Rect::new(0.0, 0.0, 50.0, 100.0))]);
        assert!(!non_leaf.is_leaf());
    }

    #[test]
    fn test_bisection_three_nodes() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let nodes: Vec<(NodeId, VisualMass)> = (0..3)
            .map(|_| (NodeId::new(), VisualMass::new(10.0)))
            .collect();

        let region = planner.plan(bounds, nodes);

        // Should recursively split
        assert!(!region.children.is_empty() || region.node_id.is_some());
    }

    #[test]
    fn test_bisection_four_nodes() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let nodes: Vec<(NodeId, VisualMass)> = (0..4)
            .map(|_| (NodeId::new(), VisualMass::new(10.0)))
            .collect();

        let region = planner.plan(bounds, nodes);

        // All nodes should be assigned
        let assigned_count = count_assigned_nodes(&region);
        assert_eq!(assigned_count, 4);
    }

    fn count_assigned_nodes(region: &FloorplanRegion) -> usize {
        if let Some(_) = region.node_id {
            1
        } else {
            region.children.iter().map(count_assigned_nodes).sum()
        }
    }

    #[test]
    fn test_bisection_empty_input() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let region = planner.plan(bounds, vec![]);

        assert!(region.node_id.is_none());
        assert!(region.children.is_empty());
        assert_eq!(region.rect, bounds);
    }

    #[test]
    fn test_bisection_extreme_aspect_ratio_wide() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 1000.0, 100.0); // Very wide

        let node1 = NodeId::new();
        let node2 = NodeId::new();

        let region = planner.plan(
            bounds,
            vec![
                (node1, VisualMass::new(10.0)),
                (node2, VisualMass::new(10.0)),
            ],
        );

        // Wide bounds should split horizontally
        assert_eq!(region.split_direction, Some(SplitDirection::Horizontal));
    }

    #[test]
    fn test_bisection_extreme_aspect_ratio_tall() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 1000.0); // Very tall

        let node1 = NodeId::new();
        let node2 = NodeId::new();

        let region = planner.plan(
            bounds,
            vec![
                (node1, VisualMass::new(10.0)),
                (node2, VisualMass::new(10.0)),
            ],
        );

        // Tall bounds should split vertically
        assert_eq!(region.split_direction, Some(SplitDirection::Vertical));
    }

    #[test]
    fn test_bisection_equal_weights() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let nodes: Vec<(NodeId, VisualMass)> = (0..4)
            .map(|_| (NodeId::new(), VisualMass::new(50.0)))
            .collect();

        let region = planner.plan(bounds, nodes);

        // With equal weights, splits should be roughly 50/50
        assert!(!region.children.is_empty());
    }

    #[test]
    fn test_bisection_unbalanced_weights() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let node1 = NodeId::new();
        let node2 = NodeId::new();

        let region = planner.plan(
            bounds,
            vec![
                (node1, VisualMass::new(90.0)), // Heavy
                (node2, VisualMass::new(10.0)), // Light
            ],
        );

        assert_eq!(region.children.len(), 2);

        // Heavy node should get more space
        let node1_region = find_node_region(&region, node1);
        let node2_region = find_node_region(&region, node2);

        if let (Some(r1), Some(r2)) = (node1_region, node2_region) {
            assert!(r1.area() > r2.area());
        }
    }

    fn find_node_region<'a>(
        region: &'a FloorplanRegion,
        node_id: NodeId,
    ) -> Option<&'a FloorplanRegion> {
        if region.node_id == Some(node_id) {
            return Some(region);
        }
        for child in &region.children {
            if let Some(found) = find_node_region(child, node_id) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn test_bisection_deep_nesting() {
        let mut planner = BisectionFloorplanner::new();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let nodes: Vec<(NodeId, VisualMass)> = (0..8)
            .map(|_| (NodeId::new(), VisualMass::new(10.0)))
            .collect();

        let region = planner.plan(bounds, nodes);

        // Should create a tree structure
        let depth = calculate_tree_depth(&region);
        assert!(depth >= 3); // 8 nodes should create at least 3 levels
    }

    fn calculate_tree_depth(region: &FloorplanRegion) -> usize {
        if region.children.is_empty() {
            1
        } else {
            1 + region
                .children
                .iter()
                .map(calculate_tree_depth)
                .max()
                .unwrap_or(0)
        }
    }

    #[test]
    fn test_bisection_with_min_region_size() {
        let mut planner = BisectionFloorplanner::new().with_min_region_size(50.0);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let nodes: Vec<(NodeId, VisualMass)> = (0..4)
            .map(|_| (NodeId::new(), VisualMass::new(10.0)))
            .collect();

        let region = planner.plan(bounds, nodes);

        // Should still produce a valid plan
        let assigned_count = count_assigned_nodes(&region);
        assert!(assigned_count > 0);
    }

    #[test]
    fn test_bisection_with_target_aspect_ratio() {
        let mut planner = BisectionFloorplanner::new().with_target_aspect_ratio(1.0);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let node1 = NodeId::new();
        let node2 = NodeId::new();

        let region = planner.plan(
            bounds,
            vec![
                (node1, VisualMass::new(10.0)),
                (node2, VisualMass::new(10.0)),
            ],
        );

        assert!(!region.children.is_empty());
    }

    #[test]
    fn test_grid_floorplanner_more_nodes_than_cells() {
        let planner = GridFloorplanner::new(2, 2);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let nodes: Vec<NodeId> = (0..10).map(|_| NodeId::new()).collect(); // 10 nodes, 4 cells
        let region = planner.plan(bounds, nodes);

        // Should only place as many nodes as there are cells
        assert_eq!(region.children.len(), 4);
    }

    #[test]
    fn test_grid_floorplanner_fewer_nodes_than_cells() {
        let planner = GridFloorplanner::new(2, 2);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

        let nodes: Vec<NodeId> = (0..2).map(|_| NodeId::new()).collect();
        let region = planner.plan(bounds, nodes);

        assert_eq!(region.children.len(), 2);
    }

    #[test]
    fn test_grid_floorplanner_positions() {
        let planner = GridFloorplanner::new(3, 2);
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);

        let nodes: Vec<NodeId> = (0..6).map(|_| NodeId::new()).collect();
        let region = planner.plan(bounds, nodes);

        // First cell should be at (0, 0)
        assert_eq!(region.children[0].rect.x(), 0.0);
        assert_eq!(region.children[0].rect.y(), 0.0);

        // Second cell should be at (100, 0)
        assert_eq!(region.children[1].rect.x(), 100.0);
        assert_eq!(region.children[1].rect.y(), 0.0);

        // Fourth cell should be at (0, 100) - second row
        assert_eq!(region.children[3].rect.x(), 0.0);
        assert_eq!(region.children[3].rect.y(), 100.0);
    }

    #[test]
    fn test_bisection_default() {
        let planner = BisectionFloorplanner::default();
        assert_eq!(planner.min_region_size, 20.0);
        assert_eq!(planner.target_aspect_ratio, 1.0);
    }
}
