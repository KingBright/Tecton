//! Tecton Routing - Smart routing layer for Tecton
//!
//! This crate provides:
//! - Grid-based graph for discretized routing space
//! - Orthogonal A* pathfinding with bend penalties
//! - Congestion detection and hotspot identification
//! - Route representation and management

pub mod congestion_scanner;
pub mod grid_graph;
pub mod path_finder;
pub mod route;

pub use congestion_scanner::{CongestionResult, CongestionScanner, Hotspot};
pub use grid_graph::{CellType, GridGraph, GridPath};
pub use path_finder::{MultiTerminalRouter, OrthogonalPathFinder, PreferredDirection};
pub use route::{Route, RouteSet, RouteStats};

use std::collections::HashMap;
use tecton_types::{
    edge::Edge,
    geometry::{Point, Rect},
    id::{EdgeId, NodeId, PortId},
};

/// Main routing engine that coordinates pathfinding and congestion management
pub struct RoutingEngine {
    path_finder: OrthogonalPathFinder,
    congestion_scanner: CongestionScanner,
    cell_size: f32,
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingEngine {
    pub fn new() -> Self {
        Self {
            path_finder: OrthogonalPathFinder::new(),
            congestion_scanner: CongestionScanner::new(),
            cell_size: 10.0,
        }
    }

    pub fn with_cell_size(mut self, cell_size: f32) -> Self {
        self.cell_size = cell_size;
        self.congestion_scanner = self.congestion_scanner.with_cell_size(cell_size);
        self
    }

    pub fn with_bend_penalty(mut self, penalty: f32) -> Self {
        self.path_finder = self.path_finder.with_bend_penalty(penalty);
        self
    }

    /// Route all edges in the diagram
    pub fn route_all(
        &mut self,
        bounds: &Rect,
        nodes: &HashMap<NodeId, Rect>,
        edges: &[Edge],
        port_positions: &HashMap<PortId, Point>,
    ) -> Result<RouteSet, String> {
        let mut grid = GridGraph::from_rect(bounds, self.cell_size);

        // Block all node areas
        for rect in nodes.values() {
            grid.block_rect(rect);
        }

        let mut routes = RouteSet::new();

        for edge in edges {
            let source_pos = port_positions
                .get(&edge.source_port)
                .ok_or_else(|| format!("Port not found: {:?}", edge.source_port))?;
            let target_pos = port_positions
                .get(&edge.target_port)
                .ok_or_else(|| format!("Port not found: {:?}", edge.target_port))?;

            if let Some(path) = self.path_finder.find_path(&grid, source_pos, target_pos) {
                // Mark path as occupied
                for &(x, y) in &path {
                    grid.mark_occupied(x, y);
                }

                let route = Route::from_grid_path(
                    edge.id,
                    edge.source_port,
                    edge.target_port,
                    &path,
                    self.cell_size,
                    *source_pos,
                    *target_pos,
                );

                routes.add_route(route);
            } else {
                return Err(format!("Could not find path for edge {:?}", edge.id));
            }
        }

        Ok(routes)
    }

    /// Scan for congestion in existing routes
    pub fn scan_congestion(
        &self,
        grid: &GridGraph,
        routes: &RouteSet,
        node_positions: &HashMap<NodeId, Rect>,
    ) -> CongestionResult {
        self.congestion_scanner
            .scan_with_routes(grid, routes, node_positions)
    }

    /// Get the path finder
    pub fn path_finder(&self) -> &OrthogonalPathFinder {
        &self.path_finder
    }

    /// Get the congestion scanner
    pub fn congestion_scanner(&self) -> &CongestionScanner {
        &self.congestion_scanner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_engine() {
        let engine = RoutingEngine::new();

        assert_eq!(engine.cell_size, 10.0);
    }
}
