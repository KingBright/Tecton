use std::collections::HashMap;

use tecton_types::{
    geometry::{Point, Rect},
    id::{EdgeId, NodeId},
};

use crate::grid_graph::{CellType, GridGraph};
use crate::route::RouteSet;

/// A congestion hotspot in the grid
#[derive(Debug, Clone, PartialEq)]
pub struct Hotspot {
    /// Grid coordinates of the hotspot
    pub grid_x: usize,
    pub grid_y: usize,
    /// Position in world coordinates
    pub position: Point,
    /// Severity level (higher = more congested)
    pub severity: f32,
    /// Number of routes passing through
    pub route_count: usize,
    /// IDs of edges passing through
    pub edge_ids: Vec<EdgeId>,
    /// Nearest node ID (for margin adjustment)
    pub nearest_node: Option<NodeId>,
}

impl Hotspot {
    pub fn new(grid_x: usize, grid_y: usize, position: Point, severity: f32) -> Self {
        Self {
            grid_x,
            grid_y,
            position,
            severity,
            route_count: 0,
            edge_ids: Vec::new(),
            nearest_node: None,
        }
    }

    pub fn with_route_count(mut self, count: usize) -> Self {
        self.route_count = count;
        self
    }

    pub fn with_edge_ids(mut self, edge_ids: Vec<EdgeId>) -> Self {
        self.edge_ids = edge_ids;
        self
    }

    pub fn with_nearest_node(mut self, node_id: NodeId) -> Self {
        self.nearest_node = Some(node_id);
        self
    }

    /// Get the recommended margin increase for this hotspot
    pub fn recommended_margin_increase(&self) -> f32 {
        // Base increase on severity
        let base_increase = self.severity * 5.0;

        // Cap at reasonable maximum
        base_increase.min(50.0)
    }
}

/// Result of congestion scanning
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CongestionResult {
    /// List of detected hotspots
    pub hotspots: Vec<Hotspot>,
    /// Maximum severity found
    pub max_severity: f32,
    /// Average severity across all hotspots
    pub avg_severity: f32,
    /// Total number of congested cells
    pub congested_cell_count: usize,
}

impl CongestionResult {
    pub fn new(hotspots: Vec<Hotspot>) -> Self {
        let max_severity = hotspots.iter().map(|h| h.severity).fold(0.0, f32::max);

        let avg_severity = if hotspots.is_empty() {
            0.0
        } else {
            hotspots.iter().map(|h| h.severity).sum::<f32>() / hotspots.len() as f32
        };

        let congested_cell_count = hotspots.len();

        Self {
            hotspots,
            max_severity,
            avg_severity,
            congested_cell_count,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hotspots.is_empty()
    }

    /// Get hotspots sorted by severity (highest first)
    pub fn sorted_by_severity(&self) -> Vec<&Hotspot> {
        let mut sorted: Vec<_> = self.hotspots.iter().collect();
        sorted.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap());
        sorted
    }

    /// Get hotspots near a specific node
    pub fn hotspots_near_node(&self, node_id: NodeId) -> Vec<&Hotspot> {
        self.hotspots
            .iter()
            .filter(|h| h.nearest_node == Some(node_id))
            .collect()
    }
}

/// Scans the grid for congestion hotspots
#[derive(Debug, Clone)]
pub struct CongestionScanner {
    /// Threshold for considering a cell congested
    pub congestion_threshold: u32,
    /// Grid cell size for scanning
    pub cell_size: f32,
    /// Minimum severity to report
    pub min_severity: f32,
}

impl Default for CongestionScanner {
    fn default() -> Self {
        Self {
            congestion_threshold: 2,
            cell_size: 10.0,
            min_severity: 0.5,
        }
    }
}

impl CongestionScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_congestion_threshold(mut self, threshold: u32) -> Self {
        self.congestion_threshold = threshold;
        self
    }

    pub fn with_cell_size(mut self, cell_size: f32) -> Self {
        self.cell_size = cell_size;
        self
    }

    pub fn with_min_severity(mut self, min_severity: f32) -> Self {
        self.min_severity = min_severity;
        self
    }

    /// Scan the grid for congestion
    pub fn scan(&self, grid: &GridGraph) -> CongestionResult {
        let mut hotspots = Vec::new();

        for y in 0..grid.height {
            for x in 0..grid.width {
                let occupancy = grid.get_occupancy(x, y);

                if occupancy >= self.congestion_threshold {
                    let cell_type = grid.get(x, y).unwrap_or(CellType::Empty);

                    // Calculate severity based on occupancy and cell type
                    let base_severity = occupancy as f32 / self.congestion_threshold as f32;
                    let type_multiplier = match cell_type {
                        CellType::Congested => 2.0,
                        CellType::Occupied => 1.5,
                        _ => 1.0,
                    };

                    let severity = base_severity * type_multiplier;

                    if severity >= self.min_severity {
                        if let Some(position) = grid.cell_to_point(x, y) {
                            hotspots.push(
                                Hotspot::new(x, y, position, severity)
                                    .with_route_count(occupancy as usize),
                            );
                        }
                    }
                }
            }
        }

        CongestionResult::new(hotspots)
    }

    /// Scan with route information to track which edges are involved
    pub fn scan_with_routes(
        &self,
        grid: &GridGraph,
        routes: &RouteSet,
        node_positions: &HashMap<NodeId, Rect>,
    ) -> CongestionResult {
        let mut hotspots = Vec::new();

        // Build a map of which routes pass through each cell
        let mut cell_routes: HashMap<(usize, usize), Vec<EdgeId>> = HashMap::new();

        for route in routes.routes() {
            for point in route.points() {
                if let Some((x, y)) = grid.point_to_cell(&point) {
                    cell_routes.entry((x, y)).or_default().push(route.edge_id);
                }
            }
        }

        for y in 0..grid.height {
            for x in 0..grid.width {
                let occupancy = grid.get_occupancy(x, y);

                if occupancy >= self.congestion_threshold {
                    let cell_type = grid.get(x, y).unwrap_or(CellType::Empty);

                    let base_severity = occupancy as f32 / self.congestion_threshold as f32;
                    let type_multiplier = match cell_type {
                        CellType::Congested => 2.0,
                        CellType::Occupied => 1.5,
                        _ => 1.0,
                    };

                    let severity = base_severity * type_multiplier;

                    if severity >= self.min_severity {
                        if let Some(position) = grid.cell_to_point(x, y) {
                            let edge_ids = cell_routes.get(&(x, y)).cloned().unwrap_or_default();

                            // Find nearest node
                            let nearest_node = self.find_nearest_node(&position, node_positions);

                            let mut hotspot = Hotspot::new(x, y, position, severity)
                                .with_route_count(occupancy as usize)
                                .with_edge_ids(edge_ids);
                            if let Some(node_id) = nearest_node {
                                hotspot = hotspot.with_nearest_node(node_id);
                            }
                            hotspots.push(hotspot);
                        }
                    }
                }
            }
        }

        CongestionResult::new(hotspots)
    }

    fn find_nearest_node(
        &self,
        position: &Point,
        node_positions: &HashMap<NodeId, Rect>,
    ) -> Option<NodeId> {
        node_positions
            .iter()
            .min_by(|(_, rect1), (_, rect2)| {
                let dist1 = position.manhattan_distance_to(&rect1.center());
                let dist2 = position.manhattan_distance_to(&rect2.center());
                dist1.partial_cmp(&dist2).unwrap()
            })
            .map(|(id, _)| *id)
    }

    /// Calculate margin adjustments based on congestion
    pub fn calculate_margin_adjustments(
        &self,
        congestion: &CongestionResult,
    ) -> HashMap<NodeId, f32> {
        let mut adjustments: HashMap<NodeId, f32> = HashMap::new();

        for hotspot in &congestion.hotspots {
            if let Some(node_id) = hotspot.nearest_node {
                let increase = hotspot.recommended_margin_increase();
                adjustments
                    .entry(node_id)
                    .and_modify(|v| *v += increase)
                    .or_insert(increase);
            }
        }

        adjustments
    }

    /// Estimate the probability of routing success
    pub fn estimate_routing_success(&self, grid: &GridGraph, routes: &RouteSet) -> f32 {
        if routes.is_empty() {
            return 1.0;
        }

        let congested_count = routes.congested_routes().len();
        let total_count = routes.len();

        1.0 - (congested_count as f32 / total_count as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_congestion_scanner() {
        let mut grid = GridGraph::new(10, 10, 10.0);

        // Create some congestion
        for i in 0..5 {
            grid.increment_occupancy(5, 5);
            grid.increment_occupancy(5, 6);
            grid.increment_occupancy(6, 5);
        }

        let scanner = CongestionScanner::new().with_congestion_threshold(3);
        let result = scanner.scan(&grid);

        assert!(!result.is_empty());
        assert!(result.max_severity > 0.0);
    }

    #[test]
    fn test_hotspot_recommended_margin() {
        let hotspot = Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0);
        let margin = hotspot.recommended_margin_increase();

        assert!(margin > 0.0);
        assert!(margin <= 50.0);
    }

    #[test]
    fn test_margin_adjustments() {
        let scanner = CongestionScanner::new();

        let hotspots = vec![
            Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0).with_nearest_node(NodeId::new()),
            Hotspot::new(6, 6, Point::new(60.0, 60.0), 1.5).with_nearest_node(NodeId::new()),
        ];

        let congestion = CongestionResult::new(hotspots);
        let adjustments = scanner.calculate_margin_adjustments(&congestion);

        assert!(!adjustments.is_empty());
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_congestion_scanner_default() {
        let scanner = CongestionScanner::default();

        assert_eq!(scanner.congestion_threshold, 2);
        assert_eq!(scanner.cell_size, 10.0);
        assert_eq!(scanner.min_severity, 0.5);
    }

    #[test]
    fn test_congestion_scanner_new() {
        let scanner = CongestionScanner::new();

        assert_eq!(scanner.congestion_threshold, 2);
    }

    #[test]
    fn test_congestion_scanner_with_cell_size() {
        let scanner = CongestionScanner::new().with_cell_size(20.0);

        assert_eq!(scanner.cell_size, 20.0);
    }

    #[test]
    fn test_congestion_scanner_with_min_severity() {
        let scanner = CongestionScanner::new().with_min_severity(1.0);

        assert_eq!(scanner.min_severity, 1.0);
    }

    #[test]
    fn test_hotspot_new() {
        let hotspot = Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0);

        assert_eq!(hotspot.grid_x, 5);
        assert_eq!(hotspot.grid_y, 5);
        assert_eq!(hotspot.position, Point::new(50.0, 50.0));
        assert_eq!(hotspot.severity, 2.0);
        assert_eq!(hotspot.route_count, 0);
        assert!(hotspot.edge_ids.is_empty());
        assert!(hotspot.nearest_node.is_none());
    }

    #[test]
    fn test_hotspot_with_route_count() {
        let hotspot = Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0).with_route_count(10);

        assert_eq!(hotspot.route_count, 10);
    }

    #[test]
    fn test_hotspot_with_edge_ids() {
        let edge_ids = vec![EdgeId::new(), EdgeId::new()];
        let hotspot =
            Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0).with_edge_ids(edge_ids.clone());

        assert_eq!(hotspot.edge_ids.len(), 2);
    }

    #[test]
    fn test_hotspot_with_nearest_node() {
        let node_id = NodeId::new();
        let hotspot = Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0).with_nearest_node(node_id);

        assert_eq!(hotspot.nearest_node, Some(node_id));
    }

    #[test]
    fn test_hotspot_recommended_margin_capped() {
        // Very high severity should be capped
        let hotspot = Hotspot::new(5, 5, Point::new(50.0, 50.0), 100.0);
        let margin = hotspot.recommended_margin_increase();

        assert_eq!(margin, 50.0); // Should be capped
    }

    #[test]
    fn test_congestion_result_new() {
        let hotspots = vec![
            Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0),
            Hotspot::new(6, 6, Point::new(60.0, 60.0), 3.0),
        ];

        let result = CongestionResult::new(hotspots);

        assert_eq!(result.hotspots.len(), 2);
        assert_eq!(result.max_severity, 3.0);
        assert_eq!(result.congested_cell_count, 2);
    }

    #[test]
    fn test_congestion_result_empty() {
        let result = CongestionResult::new(vec![]);

        assert!(result.is_empty());
        assert_eq!(result.max_severity, 0.0);
        assert_eq!(result.avg_severity, 0.0);
    }

    #[test]
    fn test_congestion_result_avg_severity() {
        let hotspots = vec![
            Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0),
            Hotspot::new(6, 6, Point::new(60.0, 60.0), 4.0),
        ];

        let result = CongestionResult::new(hotspots);

        assert!((result.avg_severity - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_congestion_result_sorted_by_severity() {
        let hotspots = vec![
            Hotspot::new(5, 5, Point::new(50.0, 50.0), 1.0),
            Hotspot::new(6, 6, Point::new(60.0, 60.0), 3.0),
            Hotspot::new(7, 7, Point::new(70.0, 70.0), 2.0),
        ];

        let result = CongestionResult::new(hotspots);
        let sorted = result.sorted_by_severity();

        assert_eq!(sorted[0].severity, 3.0);
        assert_eq!(sorted[1].severity, 2.0);
        assert_eq!(sorted[2].severity, 1.0);
    }

    #[test]
    fn test_congestion_result_hotspots_near_node() {
        let node_id = NodeId::new();

        let hotspots = vec![
            Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0).with_nearest_node(node_id),
            Hotspot::new(6, 6, Point::new(60.0, 60.0), 3.0),
            Hotspot::new(7, 7, Point::new(70.0, 70.0), 1.0).with_nearest_node(node_id),
        ];

        let result = CongestionResult::new(hotspots);
        let near_node = result.hotspots_near_node(node_id);

        assert_eq!(near_node.len(), 2);
    }

    #[test]
    fn test_congestion_result_default() {
        let result = CongestionResult::default();

        assert!(result.is_empty());
        assert!(result.hotspots.is_empty());
    }

    #[test]
    fn test_scan_no_congestion() {
        let grid = GridGraph::new(10, 10, 10.0);
        let scanner = CongestionScanner::new().with_congestion_threshold(2);

        let result = scanner.scan(&grid);

        assert!(result.is_empty());
    }

    #[test]
    fn test_scan_single_hotspot() {
        let mut grid = GridGraph::new(10, 10, 10.0);

        // Create congestion at a single point
        for _ in 0..5 {
            grid.increment_occupancy(5, 5);
        }

        let scanner = CongestionScanner::new()
            .with_congestion_threshold(2)
            .with_min_severity(0.1);

        let result = scanner.scan(&grid);

        assert_eq!(result.hotspots.len(), 1);
        assert_eq!(result.hotspots[0].grid_x, 5);
        assert_eq!(result.hotspots[0].grid_y, 5);
    }

    #[test]
    fn test_scan_threshold_boundary() {
        let mut grid = GridGraph::new(10, 10, 10.0);

        // Create exactly threshold level congestion
        grid.increment_occupancy(5, 5);
        grid.increment_occupancy(5, 5);

        let scanner = CongestionScanner::new()
            .with_congestion_threshold(2)
            .with_min_severity(0.1);

        let result = scanner.scan(&grid);

        assert_eq!(result.hotspots.len(), 1);
    }

    #[test]
    fn test_scan_below_threshold() {
        let mut grid = GridGraph::new(10, 10, 10.0);

        // Below threshold
        grid.increment_occupancy(5, 5);

        let scanner = CongestionScanner::new().with_congestion_threshold(2);

        let result = scanner.scan(&grid);

        assert!(result.is_empty());
    }

    #[test]
    fn test_scan_with_routes_empty() {
        let grid = GridGraph::new(10, 10, 10.0);
        let routes = RouteSet::new();
        let node_positions = HashMap::new();

        let scanner = CongestionScanner::new();
        let result = scanner.scan_with_routes(&grid, &routes, &node_positions);

        assert!(result.is_empty());
    }

    #[test]
    fn test_estimate_routing_success_empty() {
        let grid = GridGraph::new(10, 10, 10.0);
        let routes = RouteSet::new();

        let scanner = CongestionScanner::new();
        let success = scanner.estimate_routing_success(&grid, &routes);

        assert_eq!(success, 1.0); // Empty routes = 100% success
    }

    #[test]
    fn test_margin_adjustments_same_node() {
        let scanner = CongestionScanner::new();
        let node_id = NodeId::new();

        let hotspots = vec![
            Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0).with_nearest_node(node_id),
            Hotspot::new(6, 6, Point::new(60.0, 60.0), 1.5).with_nearest_node(node_id),
        ];

        let congestion = CongestionResult::new(hotspots);
        let adjustments = scanner.calculate_margin_adjustments(&congestion);

        // Both hotspots contribute to same node
        assert_eq!(adjustments.len(), 1);
        let margin = adjustments.get(&node_id).unwrap();
        assert!(*margin > 0.0);
    }

    #[test]
    fn test_margin_adjustments_no_nearest_node() {
        let scanner = CongestionScanner::new();

        let hotspots = vec![
            Hotspot::new(5, 5, Point::new(50.0, 50.0), 2.0),
            Hotspot::new(6, 6, Point::new(60.0, 60.0), 1.5),
        ];

        let congestion = CongestionResult::new(hotspots);
        let adjustments = scanner.calculate_margin_adjustments(&congestion);

        assert!(adjustments.is_empty());
    }
}
