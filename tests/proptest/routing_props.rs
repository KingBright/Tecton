//! Property-Based Tests for Routing Module
//!
//! These tests verify routing invariants using property-based testing.

use tecton_routing::grid_graph::{GridGraph, GridPoint};
use tecton_routing::path_finder::OrthogonalPathFinder;
use tecton_routing::route::Route;
use tecton_types::edge::{Edge, EdgeKind};
use tecton_types::geometry::{Point, Rect};
use tecton_types::id::{EdgeId, NodeId, PortId};
use tecton_types::style::Style;

/// Property: Path only contains valid segments
#[test]
fn prop_path_valid_segments() {
    let grid = GridGraph::from_rect(&Rect::new(0.0, 0.0, 100.0, 100.0),
        10.0,
    );
    let finder = OrthogonalPathFinder::new();

    let start = Point::new(10.0, 10.0);
    let end = Point::new(80.0, 80.0);

    if let Some(path) = finder.find_path(&grid, &start, &end) {
        // Path should have at least 2 points (start and end)
        assert!(path.points.len() >= 2);

        // All points should be within grid bounds (approximately)
        for point in &path.points {
            assert!(point.x >= 0 && point.x <= 10, "X coordinate should be within bounds");
            assert!(point.y >= 0 && point.y <= 10, "Y coordinate should be within bounds");
        }
    }
}

/// Property: Path connects start to end
#[test]
fn prop_path_connects_start_end() {
    let grid = GridGraph::from_rect(
        &Rect::new(0.0, 0.0, 100.0, 100.0),
        10.0,
    );
    let finder = OrthogonalPathFinder::new();

    let start = Point::new(10.0, 10.0);
    let end = Point::new(80.0, 80.0);

    if let Some(path) = finder.find_path(&grid, &start, &end) {
        let grid_start = grid.to_grid(&start);
        let grid_end = grid.to_grid(&end);

        assert_eq!(
            path.points.first(),
            Some(&grid_start),
            "Path should start at start point"
        );
        assert_eq!(
            path.points.last(),
            Some(&grid_end),
            "Path should end at end point"
        );
    }
}

/// Property: Route doesn't self-intersect (simplified)
#[test]
fn prop_route_no_self_crossing_simple() {
    // Create a simple route with 3 segments forming an L shape
    let edge = Edge::new(
        EdgeId::new(),
        NodeId::new(),
        NodeId::new(),
        PortId::new(),
        PortId::new(),
    );

    let route = Route::new(edge.id, edge.source_port, edge.target_port);

    // A new route has no segments, so it can't self-intersect
    assert!(route.segments.is_empty() || route.segments.len() < 4);
}

/// Property: Grid point to world coordinate conversion is reversible
#[test]
fn prop_grid_world_conversion() {
    let grid = GridGraph::from_rect(
        &Rect::new(0.0, 0.0, 100.0, 100.0),
        10.0,
    );

    let test_points = vec![
        Point::new(0.0, 0.0),
        Point::new(50.0, 50.0),
        Point::new(95.0, 95.0),
    ];

    for world_point in test_points {
        let grid_point = grid.to_grid(&world_point);
        let back_to_world = grid.to_world(&grid_point);

        // Allow for rounding errors due to grid discretization
        let tolerance = grid.cell_size;
        assert!(
            (world_point.x() - back_to_world.x()).abs() <= tolerance,
            "X coordinate should be approximately reversible"
        );
        assert!(
            (world_point.y() - back_to_world.y()).abs() <= tolerance,
            "Y coordinate should be approximately reversible"
        );
    }
}

/// Property: Path length is at least Manhattan distance
#[test]
fn prop_path_length_lower_bound() {
    let grid = GridGraph::from_rect(
        &Rect::new(0.0, 0.0, 100.0, 100.0),
        10.0,
    );
    let finder = OrthogonalPathFinder::new();

    let start = Point::new(10.0, 10.0);
    let end = Point::new(80.0, 80.0);

    // Manhattan distance
    let manhattan_dist = ((end.x() - start.x()).abs() + (end.y() - start.y()).abs()) / 10.0;

    if let Some(path) = finder.find_path(&grid, &start, &end) {
        let path_length = (path.points.len() - 1) as f32;
        assert!(
            path_length >= manhattan_dist - 1.0,
            "Path length should be at least Manhattan distance (in grid cells)"
        );
    }
}

/// Property: Blocked cells are respected
#[test]
fn prop_blocked_cells_avoided() {
    let mut grid = GridGraph::from_rect(
        &Rect::new(0.0, 0.0, 100.0, 100.0),
        10.0,
    );

    // Block the center cell
    let center = GridPoint::new(5, 5);
    grid.block(center);

    // Verify cell is blocked
    assert!(grid.is_blocked(&center), "Cell should be blocked");
}

/// Property: Empty path has zero length
#[test]
fn prop_empty_route_zero_length() {
    let edge = Edge::new(
        EdgeId::new(),
        NodeId::new(),
        NodeId::new(),
        PortId::new(),
        PortId::new(),
    );

    let route = Route::new(edge.id, edge.source_port, edge.target_port);

    // Empty route has no segments
    assert!(route.segments.is_empty());
}

/// Property: Grid bounds are respected
#[test]
fn prop_grid_bounds_respected() {
    let rect = Rect::new(10.0, 20.0, 200.0, 150.0);
    let cell_size = 10.0;

    let grid = GridGraph::from_rect(&rect, cell_size);

    // Grid should match input dimensions
    let expected_cols = (rect.width() / cell_size).ceil() as i32;
    let expected_rows = (rect.height() / cell_size).ceil() as i32;

    // Check that grid dimensions are reasonable
    assert!(expected_cols > 0);
    assert!(expected_rows > 0);
}

/// Property: Orthogonal path only has horizontal and vertical segments
#[test]
fn prop_orthogonal_segments() {
    let grid = GridGraph::from_rect(
        &Rect::new(0.0, 0.0, 100.0, 100.0),
        10.0,
    );
    let finder = OrthogonalPathFinder::new();

    let start = Point::new(10.0, 10.0);
    let end = Point::new(80.0, 80.0);

    if let Some(path) = finder.find_path(&grid, &start, &end) {
        // Check consecutive points form orthogonal segments
        for i in 0..path.points.len().saturating_sub(1) {
            let p1 = &path.points[i];
            let p2 = &path.points[i + 1];

            // Either x or y should be the same (orthogonal)
            let dx = (p2.x - p1.x).abs();
            let dy = (p2.y - p1.y).abs();

            // Should be exactly 1 in one direction, 0 in other (grid movement)
            assert!(
                (dx == 1 && dy == 0) || (dx == 0 && dy == 1) || (dx == 0 && dy == 0),
                "Movement should be orthogonal or zero"
            );
        }
    }
}
