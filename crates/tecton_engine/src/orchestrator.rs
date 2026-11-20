use std::collections::HashMap;

use tecton_layout::{
    floorplanner::BisectionFloorplanner,
    layout_context::{LayoutNode as ComputedLayoutNode, LayoutResult},
    mass_calculator::MassCalculator,
};
use tecton_render::Renderer;
use tecton_routing::{
    congestion_scanner::CongestionScanner,
    grid_graph::GridGraph,
    path_finder::OrthogonalPathFinder,
    route::{Route, RouteSet},
    RoutingEngine,
};
use tecton_types::{
    error::{Result, TectonError},
    geometry::Port,
    geometry::{Point, PortSide, Rect},
    id::{NodeId, PortId},
    mass::VisualMass,
    style::Style,
};

use crate::layout_graph::{LayoutGraph, LayoutNode};

/// Configuration for the layout and routing engine
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum iterations for congestion feedback loop
    pub max_iterations: usize,
    /// Grid cell size for routing
    pub routing_cell_size: f32,
    /// Bend penalty for pathfinding
    pub bend_penalty: f32,
    /// Congestion threshold
    pub congestion_threshold: u32,
    /// Step size for margin expansion
    pub margin_step_size: f32,
    /// Canvas bounds
    pub canvas_bounds: Rect,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            routing_cell_size: 10.0,
            bend_penalty: 10.0,
            congestion_threshold: 2,
            margin_step_size: 5.0,
            canvas_bounds: Rect::new(0.0, 0.0, 800.0, 600.0),
        }
    }
}

/// The main orchestrator that coordinates layout and routing
pub struct Orchestrator {
    config: OrchestratorConfig,
    graph: LayoutGraph,
    computed_layout: Option<LayoutResult>,
    routes: Option<RouteSet>,
    margin_map: HashMap<NodeId, f32>,
}

impl Orchestrator {
    pub fn new(graph: LayoutGraph) -> Self {
        Self {
            config: OrchestratorConfig::default(),
            graph,
            computed_layout: None,
            routes: None,
            margin_map: HashMap::new(),
        }
    }

    pub fn with_config(mut self, config: OrchestratorConfig) -> Self {
        self.config = config;
        self
    }

    /// Run the complete layout and routing pipeline
    pub fn run(&mut self) -> Result<(LayoutResult, RouteSet)> {
        // Phase 1: Calculate visual masses
        let masses = self.calculate_masses();

        // Phase 2: Macro layout (floorplanning)
        let floorplan = self.perform_floorplan(&masses)?;

        let mut iteration = 0;
        let mut final_routes = RouteSet::new();
        let mut final_layout = LayoutResult::new(
            ComputedLayoutNode::new(NodeId::new(), self.config.canvas_bounds, Style::default()),
            self.config.canvas_bounds,
        );

        // We run the Micro layout & Routing iteratively to resolve congestions.
        loop {
            // Phase 3: Micro layout (flexbox within regions) using Taffy
            let layout = self.perform_micro_layout(&floorplan, &self.margin_map)?;
            self.computed_layout = Some(layout.clone());
            final_layout = layout.clone();

            // Phase 4: Construct Routing grid
            let node_bounds = self.collect_node_bounds(&layout);
            let port_positions = self.collect_port_positions(&layout, &node_bounds)?;

            let mut grid =
                GridGraph::from_rect(&self.config.canvas_bounds, self.config.routing_cell_size);

            let routing_margin = self.config.routing_cell_size * 2.0;
            for rect in node_bounds.values() {
                // Inset the blocked area to leave room for routing around nodes
                let inset_rect = Rect::new(
                    rect.x() + routing_margin,
                    rect.y() + routing_margin,
                    (rect.width() - 2.0 * routing_margin).max(0.0),
                    (rect.height() - 2.0 * routing_margin).max(0.0),
                );
                if inset_rect.width() > 0.0 && inset_rect.height() > 0.0 {
                    grid.block_rect(&inset_rect);
                }
            }

            // Phase 5: Route all edges
            let routes = self.route_edges(&grid, &port_positions, &node_bounds)?;
            final_routes = routes.clone();

            // Check for convergence
            if iteration >= self.config.max_iterations {
                break;
            }

            // Phase 6: Scan for congestion
            let scanner = CongestionScanner::new()
                .with_congestion_threshold(self.config.congestion_threshold);
            let congestion = scanner.scan_with_routes(&grid, &routes, &node_bounds);

            if congestion.is_empty() {
                break;
            }

            // Expand margins for congested areas, triggering a layout change in next iteration
            self.expand_margins(&congestion);

            iteration += 1;
        }

        self.routes = Some(final_routes.clone());
        Ok((final_layout, final_routes))
    }

    /// Calculate visual masses for all nodes
    fn calculate_masses(&self) -> HashMap<NodeId, VisualMass> {
        let mut masses = HashMap::new();
        let mut calculator = MassCalculator::new();

        for node in self.graph.nodes() {
            // Simple mass calculation based on label length and children
            let label_mass = node.label.len() as f32 * 0.5;
            let children_mass = node.children.len() as f32 * 10.0;
            let port_mass = node.ports.len() as f32 * 2.0;

            let mass = VisualMass::new(10.0)
                .with_content(label_mass)
                .with_children(children_mass)
                .with_connectivity(port_mass);

            masses.insert(node.id, mass);
        }

        masses
    }

    /// Perform macro layout using weighted recursive bisection
    fn perform_floorplan(&self, masses: &HashMap<NodeId, VisualMass>) -> Result<FloorplanResult> {
        let mut floorplanner = BisectionFloorplanner::new();

        // Collect nodes with masses: ONLY TOP LEVEL NODES for Floorplanning
        let nodes_with_mass: Vec<(NodeId, VisualMass)> = self
            .graph
            .nodes()
            .iter()
            .filter(|n| n.parent.is_none())
            .filter_map(|n| masses.get(&n.id).map(|m| (n.id, *m)))
            .collect();

        let region = floorplanner.plan(self.config.canvas_bounds, nodes_with_mass);

        // Extract node_regions from the floorplan
        let node_regions = self.extract_node_regions(&region);

        Ok(FloorplanResult {
            region,
            node_regions,
        })
    }

    /// Extract node to region mapping from floorplan
    fn extract_node_regions(
        &self,
        region: &tecton_layout::floorplanner::FloorplanRegion,
    ) -> HashMap<NodeId, Rect> {
        let mut regions = HashMap::new();
        self.collect_node_regions_recursive(region, &mut regions);
        regions
    }

    fn collect_node_regions_recursive(
        &self,
        region: &tecton_layout::floorplanner::FloorplanRegion,
        regions: &mut HashMap<NodeId, Rect>,
    ) {
        if let Some(node_id) = region.node_id {
            regions.insert(node_id, region.rect);
        }
        for child in &region.children {
            self.collect_node_regions_recursive(child, regions);
        }
    }

    /// Perform micro layout using Taffy flexbox
    fn perform_micro_layout(
        &self,
        floorplan: &FloorplanResult,
        margin_map: &HashMap<NodeId, f32>,
    ) -> Result<LayoutResult> {
        let mut root_node =
            ComputedLayoutNode::new(NodeId::new(), self.config.canvas_bounds, Style::default());

        // For each top level region from floorplanner, compute its inner subtree
        for (node_id, rect) in &floorplan.node_regions {
            if let Some(graph_node) = self.graph.get_node(*node_id) {
                let mut taffy_tree = taffy::TaffyTree::new();
                let mut id_map = HashMap::new();

                let root_taffy_id = self.build_taffy_tree_recursive(
                    graph_node,
                    &mut taffy_tree,
                    &mut id_map,
                    margin_map,
                )?;

                let root_size = taffy::Size {
                    width: taffy::AvailableSpace::Definite(rect.width()),
                    height: taffy::AvailableSpace::Definite(rect.height()),
                };

                taffy_tree
                    .compute_layout(root_taffy_id, root_size)
                    .map_err(|e| TectonError::layout(e.to_string()))?;

                let child_layout = self.extract_taffy_layout(
                    graph_node,
                    &taffy_tree,
                    &id_map,
                    root_taffy_id,
                    *rect,
                )?;
                root_node.children.push(child_layout);
            }
        }

        Ok(LayoutResult::new(root_node, self.config.canvas_bounds))
    }

    fn build_taffy_tree_recursive(
        &self,
        node: &LayoutNode,
        tree: &mut taffy::TaffyTree,
        id_map: &mut HashMap<taffy::NodeId, NodeId>,
        margin_map: &HashMap<NodeId, f32>,
    ) -> Result<taffy::NodeId> {
        let mut style = self.convert_tecton_style(&node.style);

        // Apply congestion margin expansions if any
        if let Some(margin_inc) = margin_map.get(&node.id) {
            style.margin.left = taffy::style_helpers::length(node.style.margin.left() + margin_inc);
            style.margin.right =
                taffy::style_helpers::length(node.style.margin.right() + margin_inc);
            style.margin.top = taffy::style_helpers::length(node.style.margin.top() + margin_inc);
            style.margin.bottom =
                taffy::style_helpers::length(node.style.margin.bottom() + margin_inc);
        }

        if node.children.is_empty() {
            // Leaf node size estimation
            let (label_width, label_height) =
                tecton_layout::layout_context::estimate_leaf_label_size(&node.label);
            style.size = taffy::Size {
                width: taffy::Dimension::Length(label_width),
                height: taffy::Dimension::Length(label_height),
            };
            let t_id = tree
                .new_leaf(style)
                .map_err(|e| TectonError::layout(e.to_string()))?;
            id_map.insert(t_id, node.id);
            Ok(t_id)
        } else {
            // Container node
            let mut child_ids = Vec::new();
            for child_id in &node.children {
                if let Some(child_node) = self.graph.get_node(*child_id) {
                    let child_t_id =
                        self.build_taffy_tree_recursive(child_node, tree, id_map, margin_map)?;
                    child_ids.push(child_t_id);
                }
            }
            let t_id = tree
                .new_with_children(style, &child_ids)
                .map_err(|e| TectonError::layout(e.to_string()))?;
            id_map.insert(t_id, node.id);
            Ok(t_id)
        }
    }

    fn extract_taffy_layout(
        &self,
        node: &LayoutNode,
        tree: &taffy::TaffyTree,
        id_map: &HashMap<taffy::NodeId, NodeId>,
        t_id: taffy::NodeId,
        parent_rect: Rect,
    ) -> Result<ComputedLayoutNode> {
        let layout = tree
            .layout(t_id)
            .map_err(|e| TectonError::layout(e.to_string()))?;

        let rect = Rect::new(
            parent_rect.x() + layout.location.x,
            parent_rect.y() + layout.location.y,
            layout.size.width,
            layout.size.height,
        );

        let mut computed_node = ComputedLayoutNode::new(node.id, rect, node.style.clone())
            .with_label(node.label.clone());

        for child_t_id in tree
            .children(t_id)
            .map_err(|e| TectonError::layout(e.to_string()))?
        {
            if let Some(child_id) = id_map.get(&child_t_id) {
                if let Some(child_node) = self.graph.get_node(*child_id) {
                    let child_computed =
                        self.extract_taffy_layout(child_node, tree, id_map, child_t_id, rect)?;
                    computed_node.children.push(child_computed);
                }
            }
        }

        Ok(computed_node)
    }

    fn convert_tecton_style(&self, style: &Style) -> taffy::Style {
        use taffy::style_helpers::length;
        use tecton_types::style::FlexDirection;

        taffy::Style {
            display: taffy::Display::Flex,
            flex_direction: match style.flex_direction {
                FlexDirection::Row => taffy::FlexDirection::Row,
                FlexDirection::Column => taffy::FlexDirection::Column,
                FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
                FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
            },
            padding: taffy::Rect {
                left: length(style.padding.left()),
                right: length(style.padding.right()),
                top: length(style.padding.top()),
                bottom: length(style.padding.bottom()),
            },
            margin: taffy::Rect {
                left: length(style.margin.left()),
                right: length(style.margin.right()),
                top: length(style.margin.top()),
                bottom: length(style.margin.bottom()),
            },
            gap: taffy::Size {
                width: length(style.gap()),
                height: length(style.gap()),
            },
            ..Default::default()
        }
    }

    fn collect_node_bounds(&self, layout: &LayoutResult) -> HashMap<NodeId, Rect> {
        let mut bounds = HashMap::new();

        // Collect from computed layout
        if layout.node_map().get(&layout.root.id).is_some() {
            self.collect_bounds_recursive(&layout.root, &mut bounds);
        }

        // Filter out the root node (canvas) which covers the entire area
        let canvas_rect = self.config.canvas_bounds;
        bounds.retain(|_node_id, rect| {
            // Keep only nodes that don't cover the entire canvas
            !(rect.left() <= canvas_rect.left() + 0.1
                && rect.right() >= canvas_rect.right() - 0.1
                && rect.top() <= canvas_rect.top() + 0.1
                && rect.bottom() >= canvas_rect.bottom() - 0.1)
        });

        bounds
    }

    fn collect_bounds_recursive(
        &self,
        node: &ComputedLayoutNode,
        bounds: &mut HashMap<NodeId, Rect>,
    ) {
        bounds.insert(node.id, node.rect);
        for child in &node.children {
            self.collect_bounds_recursive(child, bounds);
        }
    }

    fn collect_port_positions(
        &self,
        layout: &LayoutResult,
        node_bounds: &HashMap<NodeId, Rect>,
    ) -> Result<HashMap<PortId, Point>> {
        let mut positions = HashMap::new();
        let routing_margin = self.config.routing_cell_size * 1.5; // Margin to keep ports routable

        for node in self.graph.nodes() {
            if let Some(bounds) = node_bounds.get(&node.id) {
                for (port_id, port) in &node.ports {
                    // Calculate port position on the edge of the node
                    let shape_geom = tecton_types::shape::ShapeFactory::create(node.shape, bounds.width(), bounds.height());
                    let direction = match port.position.side {
                        PortSide::North => tecton_types::shape::Direction::Top,
                        PortSide::South => tecton_types::shape::Direction::Bottom,
                        PortSide::East => tecton_types::shape::Direction::Right,
                        PortSide::West => tecton_types::shape::Direction::Left,
                    };
                    let pos = shape_geom.get_port_position(bounds.center(), direction);

                    // Offset the port position slightly outside the node to ensure routability
                    let offset_pos = match port.position.side {
                        PortSide::North => Point::new(pos.x(), pos.y() - routing_margin),
                        PortSide::South => Point::new(pos.x(), pos.y() + routing_margin),
                        PortSide::East => Point::new(pos.x() + routing_margin, pos.y()),
                        PortSide::West => Point::new(pos.x() - routing_margin, pos.y()),
                    };

                    positions.insert(*port_id, offset_pos);
                }
            }
        }

        Ok(positions)
    }

    fn route_edges(
        &self,
        grid: &GridGraph,
        port_positions: &HashMap<PortId, Point>,
        node_bounds: &HashMap<NodeId, Rect>,
    ) -> Result<RouteSet> {
        let mut routes = RouteSet::new();
        let path_finder = OrthogonalPathFinder::new().with_bend_penalty(self.config.bend_penalty);

        for edge in self.graph.edges() {
            // Find source and target nodes to determine best ports
            let source_node_opt = self.find_node_by_port(edge.source_port);
            let target_node_opt = self.find_node_by_port(edge.target_port);

            let (source_pos, target_pos, source_side, target_side) =
                match (source_node_opt, target_node_opt) {
                    (Some(source_node_id), Some(target_node_id)) => {
                        // Get nodes
                        let source_node = match self.graph.get_node(source_node_id) {
                            Some(n) => n,
                            None => continue,
                        };
                        let target_node = match self.graph.get_node(target_node_id) {
                            Some(n) => n,
                            None => continue,
                        };

                        // Get node bounds
                        let source_bounds = match node_bounds.get(&source_node.id) {
                            Some(b) => b,
                            None => continue,
                        };
                        let target_bounds = match node_bounds.get(&target_node.id) {
                            Some(b) => b,
                            None => continue,
                        };

                        // Select appropriate ports based on relative positions
                        let source_side =
                            Self::select_source_port_side(source_bounds, target_bounds);
                        let target_side = source_side.opposite();

                        // Find the port on the selected side
                        let source_port = source_node
                            .ports
                            .values()
                            .find(|p| p.position.side == source_side)
                            .map(|p| p.id)
                            .or_else(|| source_node.ports.keys().next().copied());
                        let target_port = target_node
                            .ports
                            .values()
                            .find(|p| p.position.side == target_side)
                            .map(|p| p.id)
                            .or_else(|| target_node.ports.keys().next().copied());

                        match (source_port, target_port) {
                            (Some(sp), Some(tp)) => {
                                let sp_pos = port_positions
                                    .get(&sp)
                                    .copied()
                                    .unwrap_or_else(|| source_bounds.center());
                                let tp_pos = port_positions
                                    .get(&tp)
                                    .copied()
                                    .unwrap_or_else(|| target_bounds.center());
                                (sp_pos, tp_pos, source_side, target_side)
                            }
                            _ => continue, // Skip this edge if ports not found
                        }
                    }
                    _ => continue, // Skip if nodes not found
                };

            // Try A* routing first
            if let Some(path) = path_finder.find_path(grid, &source_pos, &target_pos) {
                let mut route = Route::from_grid_path(
                    edge.id,
                    edge.source_port,
                    edge.target_port,
                    &path,
                    self.config.routing_cell_size,
                    source_pos,
                    target_pos,
                );

                route = route.with_edge_kind(edge.kind).with_edge_color(edge.color);

                if let Some(ref label) = edge.label {
                    route = route.with_label(label.clone());
                }

                routes.add_route(route);
            } else {
                // A* failed - create an orthogonal L-shaped or Z-shaped path manually
                let path = self.create_orthogonal_path(
                    &source_pos,
                    &target_pos,
                    source_side,
                    target_side,
                    grid,
                );

                let mut route = Route::from_grid_path(
                    edge.id,
                    edge.source_port,
                    edge.target_port,
                    &path,
                    self.config.routing_cell_size,
                    source_pos,
                    target_pos,
                );

                route = route.with_edge_kind(edge.kind).with_edge_color(edge.color);

                if let Some(ref label) = edge.label {
                    route = route.with_label(label.clone());
                }

                routes.add_route(route);
            }
        }

        Ok(routes)
    }

    /// Create an orthogonal path when A* fails
    fn create_orthogonal_path(
        &self,
        start: &Point,
        end: &Point,
        start_side: PortSide,
        _end_side: PortSide,
        _grid: &GridGraph,
    ) -> Vec<(usize, usize)> {
        let cell_size = self.config.routing_cell_size;

        // Create an L-shaped or Z-shaped path
        let sx = (start.x() / cell_size) as usize;
        let sy = (start.y() / cell_size) as usize;
        let ex = (end.x() / cell_size) as usize;
        let ey = (end.y() / cell_size) as usize;

        let mut path = Vec::new();
        path.push((sx, sy));

        // Determine path shape based on start side
        match start_side {
            PortSide::East | PortSide::West => {
                // Horizontal first, then vertical
                // Add intermediate point for Z-shape if needed
                let mid_x = (sx + ex) / 2;
                if sx != ex {
                    path.push((mid_x, sy));
                }
                if sy != ey {
                    path.push((mid_x, ey));
                }
                if ex != mid_x {
                    path.push((ex, ey));
                }
            }
            PortSide::North | PortSide::South => {
                // Vertical first, then horizontal
                let mid_y = (sy + ey) / 2;
                if sy != ey {
                    path.push((sx, mid_y));
                }
                if sx != ex {
                    path.push((ex, mid_y));
                }
                if ey != mid_y {
                    path.push((ex, ey));
                }
            }
        }

        // Ensure we have at least start and end
        if path.len() == 1 {
            path.push((ex, ey));
        }

        path
    }

    /// Find the node that contains a given port
    fn find_node_by_port(&self, port_id: PortId) -> Option<NodeId> {
        for node in self.graph.nodes() {
            if node.ports.contains_key(&port_id) {
                return Some(node.id);
            }
        }
        None
    }

    /// Select the appropriate source port side based on relative positions
    fn select_source_port_side(source: &Rect, target: &Rect) -> tecton_types::geometry::PortSide {
        use tecton_types::geometry::PortSide;

        let dx = target.center().x() - source.center().x();
        let dy = target.center().y() - source.center().y();

        // Choose based on which direction dominates
        if dx.abs() > dy.abs() {
            // Horizontal dominates
            if dx > 0.0 {
                PortSide::East // Target is to the right
            } else {
                PortSide::West // Target is to the left
            }
        } else {
            // Vertical dominates
            if dy > 0.0 {
                PortSide::South // Target is below
            } else {
                PortSide::North // Target is above
            }
        }
    }

    fn expand_margins(
        &mut self,
        congestion: &tecton_routing::congestion_scanner::CongestionResult,
    ) {
        for hotspot in &congestion.hotspots {
            if let Some(node_id) = hotspot.nearest_node {
                let increase = hotspot.severity * self.config.margin_step_size;
                self.margin_map
                    .entry(node_id)
                    .and_modify(|v| *v += increase)
                    .or_insert(increase);
            }
        }
    }

    /// Render the diagram to SVG
    pub fn render(&self) -> Result<String> {
        let layout = self
            .computed_layout
            .as_ref()
            .ok_or_else(|| TectonError::layout("No layout computed"))?;

        let mut renderer = Renderer::svg(
            self.config.canvas_bounds.width(),
            self.config.canvas_bounds.height(),
        );

        renderer.render(layout, self.routes.as_ref())
    }

    /// Save the rendered diagram to a file
    pub fn save(&self, path: &str) -> Result<()> {
        let svg = self.render()?;
        std::fs::write(path, svg).map_err(TectonError::IoError)?;
        Ok(())
    }

    /// Get the computed layout
    pub fn layout(&self) -> Option<&LayoutResult> {
        self.computed_layout.as_ref()
    }

    /// Get the computed routes
    pub fn routes(&self) -> Option<&RouteSet> {
        self.routes.as_ref()
    }
}

/// Result of floorplanning
struct FloorplanResult {
    region: tecton_layout::floorplanner::FloorplanRegion,
    node_regions: HashMap<NodeId, Rect>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_graph::{LayoutNode, LayoutPort};

    #[test]
    fn test_orchestrator_creation() {
        let graph = LayoutGraph::new();
        let orchestrator = Orchestrator::new(graph);

        assert!(orchestrator.layout().is_none());
        assert!(orchestrator.routes().is_none());
    }

    #[test]
    fn test_orchestrator_with_config() {
        let graph = LayoutGraph::new();
        let config = OrchestratorConfig {
            max_iterations: 5,
            routing_cell_size: 5.0,
            bend_penalty: 20.0,
            congestion_threshold: 3,
            margin_step_size: 10.0,
            canvas_bounds: Rect::new(0.0, 0.0, 400.0, 300.0),
        };

        let orchestrator = Orchestrator::new(graph).with_config(config);
        assert_eq!(orchestrator.config.max_iterations, 5);
    }

    #[test]
    fn test_orchestrator_empty_graph() {
        let graph = LayoutGraph::new();
        let mut orchestrator = Orchestrator::new(graph);

        let result = orchestrator.run();
        assert!(result.is_ok());
    }

    #[test]
    fn test_orchestrator_single_node() {
        let mut graph = LayoutGraph::new();

        let node = LayoutNode::new(NodeId::new(), "Single");
        graph.add_node(node);

        let mut orchestrator = Orchestrator::new(graph);
        let result = orchestrator.run();

        assert!(result.is_ok());
        assert!(orchestrator.layout().is_some());
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();

        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.routing_cell_size, 10.0);
        assert_eq!(config.bend_penalty, 10.0);
        assert_eq!(config.congestion_threshold, 2);
    }

    #[test]
    fn test_orchestrator_custom_canvas_bounds() {
        let graph = LayoutGraph::new();
        let config = OrchestratorConfig {
            canvas_bounds: Rect::new(0.0, 0.0, 1920.0, 1080.0),
            ..Default::default()
        };

        let orchestrator = Orchestrator::new(graph).with_config(config);
        assert_eq!(orchestrator.config.canvas_bounds.width(), 1920.0);
        assert_eq!(orchestrator.config.canvas_bounds.height(), 1080.0);
    }

    #[test]
    fn test_orchestrator_run_complete_flow() {
        let mut graph = LayoutGraph::new();

        let node1_id = NodeId::new();
        let mut node1 = LayoutNode::new(node1_id, "Node1");
        let port1 = PortId::new();
        node1.add_port(LayoutPort::new(
            port1,
            "out",
            tecton_types::geometry::PortSide::East,
        ));

        let node2_id = NodeId::new();
        let mut node2 = LayoutNode::new(node2_id, "Node2");
        let port2 = PortId::new();
        node2.add_port(LayoutPort::new(
            port2,
            "in",
            tecton_types::geometry::PortSide::West,
        ));

        graph.add_node(node1);
        graph.add_node(node2);

        let mut orchestrator = Orchestrator::new(graph);
        let result = orchestrator.run();

        assert!(result.is_ok());
        assert!(orchestrator.layout().is_some());
    }

    #[test]
    fn test_orchestrator_render_before_run() {
        let graph = LayoutGraph::new();
        let orchestrator = Orchestrator::new(graph);

        let result = orchestrator.render();
        assert!(result.is_err());
    }

    #[test]
    fn test_orchestrator_iteration_limit() {
        let graph = LayoutGraph::new();
        let config = OrchestratorConfig {
            max_iterations: 0, // Zero iterations
            ..Default::default()
        };

        let mut orchestrator = Orchestrator::new(graph).with_config(config);
        let result = orchestrator.run();

        assert!(result.is_ok());
    }

    #[test]
    fn test_orchestrator_masses_calculation() {
        let mut graph = LayoutGraph::new();

        let node1 = LayoutNode::new(NodeId::new(), "A");
        let node2 = LayoutNode::new(NodeId::new(), "This is a very long label");

        graph.add_node(node1);
        graph.add_node(node2);

        let mut orchestrator = Orchestrator::new(graph);
        let result = orchestrator.run();

        assert!(result.is_ok());
    }

    #[test]
    fn test_orchestrator_extract_node_regions() {
        let mut graph = LayoutGraph::new();

        let node1 = LayoutNode::new(NodeId::new(), "A");
        let node2 = LayoutNode::new(NodeId::new(), "B");

        graph.add_node(node1);
        graph.add_node(node2);

        let mut orchestrator = Orchestrator::new(graph);
        let result = orchestrator.run();

        assert!(result.is_ok());
        assert!(orchestrator.layout().is_some());
    }

    #[test]
    fn test_orchestrator_with_children() {
        let mut graph = LayoutGraph::new();

        let parent_id = NodeId::new();
        let child_id = NodeId::new();

        let mut parent = LayoutNode::new(parent_id, "Parent");
        parent.children.push(child_id);

        let child = LayoutNode::new(child_id, "Child").with_parent(parent_id);

        graph.add_node(parent);
        graph.add_node(child);

        let mut orchestrator = Orchestrator::new(graph);
        let result = orchestrator.run();

        assert!(result.is_ok());
    }
}
