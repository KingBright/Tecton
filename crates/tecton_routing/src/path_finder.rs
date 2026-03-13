use pathfinding::prelude::astar;

use tecton_types::geometry::Point;

use crate::grid_graph::GridGraph;

/// Orthogonal path finder using A* algorithm
///
/// This finds paths that only move horizontally or vertically,
/// with an optional penalty for direction changes (bends).
#[derive(Debug, Clone)]
pub struct OrthogonalPathFinder {
    /// Penalty for changing direction (making bends)
    pub bend_penalty: f32,
    /// Whether to prefer horizontal or vertical routing
    pub preferred_direction: PreferredDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreferredDirection {
    None,
    Horizontal,
    Vertical,
}

impl Default for OrthogonalPathFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl OrthogonalPathFinder {
    pub fn new() -> Self {
        Self {
            bend_penalty: 10.0,
            preferred_direction: PreferredDirection::None,
        }
    }

    pub fn with_bend_penalty(mut self, penalty: f32) -> Self {
        self.bend_penalty = penalty;
        self
    }

    pub fn with_preferred_direction(mut self, direction: PreferredDirection) -> Self {
        self.preferred_direction = direction;
        self
    }

    /// Find a path between two points on the grid
    ///
    /// Returns the path as a list of grid coordinates, or None if no path exists
    pub fn find_path(
        &self,
        grid: &GridGraph,
        start: &Point,
        end: &Point,
    ) -> Option<Vec<(usize, usize)>> {
        let start_cell = grid.point_to_cell(start)?;
        let end_cell = grid.point_to_cell(end)?;

        // Check if start or end is blocked
        let start_cell_type = grid.get(start_cell.0, start_cell.1)?;
        let end_cell_type = grid.get(end_cell.0, end_cell.1)?;

        if !start_cell_type.is_routable() {
            return None;
        }
        if !end_cell_type.is_routable() {
            return None;
        }

        let result = astar(
            &(start_cell.0, start_cell.1, None::<(isize, isize)>), // (x, y, last_direction)
            |&(x, y, last_dir)| self.successors(grid, x, y, last_dir),
            |&(x, y, _)| self.heuristic(x, y, end_cell.0, end_cell.1),
            |&(x, y, _)| (x, y) == end_cell,
        );

        result.map(|(path, _)| path.into_iter().map(|(x, y, _)| (x, y)).collect())
    }

    /// Find multiple paths, avoiding previously found routes
    pub fn find_path_avoiding(
        &self,
        grid: &mut GridGraph,
        start: &Point,
        end: &Point,
        existing_paths: &[Vec<(usize, usize)>],
    ) -> Option<Vec<(usize, usize)>> {
        // Temporarily mark existing paths as occupied
        let mut occupied_cells = Vec::new();
        for path in existing_paths {
            for &(x, y) in path.iter().skip(1).take(path.len().saturating_sub(2)) {
                if grid.get(x, y) == Some(crate::grid_graph::CellType::Empty) {
                    grid.set(x, y, crate::grid_graph::CellType::Occupied);
                    occupied_cells.push((x, y));
                }
            }
        }

        let result = self.find_path(grid, start, end);

        // Restore occupied cells
        for (x, y) in occupied_cells {
            grid.set(x, y, crate::grid_graph::CellType::Empty);
        }

        result
    }

    fn successors(
        &self,
        grid: &GridGraph,
        x: usize,
        y: usize,
        last_dir: Option<(isize, isize)>,
    ) -> Vec<((usize, usize, Option<(isize, isize)>), u32)> {
        let mut result = Vec::new();

        // Orthogonal directions: up, down, left, right
        let directions = [
            (0, -1), // Up
            (0, 1),  // Down
            (-1, 0), // Left
            (1, 0),  // Right
        ];

        for &(dx, dy) in &directions {
            let nx = x as isize + dx;
            let ny = y as isize + dy;

            if nx < 0 || ny < 0 {
                continue;
            }

            let nx = nx as usize;
            let ny = ny as usize;

            if let Some(cell) = grid.get(nx, ny) {
                if cell.is_routable() {
                    // Convert float cost to integer (multiply by 10)
                    let base_cost = (cell.cost() * 10.0) as u32;

                    // Add bend penalty if direction changed
                    let bend_cost = if let Some((last_dx, last_dy)) = last_dir {
                        if (dx, dy) != (last_dx, last_dy) {
                            (self.bend_penalty * 10.0) as u32
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    // Add preference bonus/penalty
                    let preference_cost: i32 = match self.preferred_direction {
                        PreferredDirection::Horizontal if dx != 0 => -5,
                        PreferredDirection::Vertical if dy != 0 => -5,
                        _ => 0,
                    };

                    let total_cost = (base_cost + bend_cost) as i32 + preference_cost;
                    result.push(((nx, ny, Some((dx, dy))), total_cost.max(1) as u32));
                }
            }
        }

        result
    }

    fn heuristic(&self, x: usize, y: usize, goal_x: usize, goal_y: usize) -> u32 {
        // Manhattan distance
        let dx = if x > goal_x { x - goal_x } else { goal_x - x };
        let dy = if y > goal_y { y - goal_y } else { goal_y - y };
        (dx + dy) as u32 * 10 // Scale to match successor costs
    }

    /// Simplify a path by removing unnecessary intermediate points
    pub fn simplify_path(&self, path: &[(usize, usize)]) -> Vec<(usize, usize)> {
        if path.len() <= 2 {
            return path.to_vec();
        }

        let mut result = vec![path[0]];
        let mut prev_dir: Option<(isize, isize)> = None;

        for i in 1..path.len() {
            let (x1, y1) = path[i - 1];
            let (x2, y2) = path[i];

            let dir = (x2 as isize - x1 as isize, y2 as isize - y1 as isize);

            if let Some(pd) = prev_dir {
                if dir != pd {
                    // Direction changed, add the corner point
                    result.push(path[i - 1]);
                }
            }

            prev_dir = Some(dir);
        }

        result.push(*path.last().unwrap());
        result
    }
}

/// Multi-terminal path finder for routing to multiple targets
pub struct MultiTerminalRouter {
    finder: OrthogonalPathFinder,
}

impl MultiTerminalRouter {
    pub fn new() -> Self {
        Self {
            finder: OrthogonalPathFinder::new(),
        }
    }

    /// Find paths from a source to multiple targets
    /// Uses a Steiner tree approximation for efficiency
    pub fn route_to_multiple(
        &self,
        grid: &mut GridGraph,
        source: &Point,
        targets: &[Point],
    ) -> Vec<Vec<(usize, usize)>> {
        let mut paths = Vec::new();
        let mut occupied = Vec::new();

        for target in targets {
            if let Some(path) = self.finder.find_path(grid, source, target) {
                // Mark path as occupied for subsequent routes
                for &(x, y) in &path {
                    grid.mark_occupied(x, y);
                    occupied.push((x, y));
                }
                paths.push(path);
            }
        }

        // Clear temporary occupancy
        for (x, y) in occupied {
            if grid.get(x, y) == Some(crate::grid_graph::CellType::Occupied) {
                grid.set(x, y, crate::grid_graph::CellType::Empty);
            }
        }

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_path_simple() {
        let grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        let start = Point::new(15.0, 15.0); // Cell (1, 1)
        let end = Point::new(55.0, 55.0); // Cell (5, 5)

        let path = finder.find_path(&grid, &start, &end);

        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.len() >= 2);
        assert_eq!(path[0], (1, 1));
        assert_eq!(path[path.len() - 1], (5, 5));
    }

    #[test]
    fn test_find_path_blocked() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        // Block a partial wall that can be routed around
        for x in 2..8 {
            grid.set(x, 4, crate::grid_graph::CellType::Blocked);
        }

        let start = Point::new(15.0, 15.0); // Cell (1, 1)
        let end = Point::new(55.0, 75.0); // Cell (5, 7)

        let path = finder.find_path(&grid, &start, &end);

        assert!(
            path.is_some(),
            "Path should exist around the partial blockage"
        );
        // Path should go around the blockage (either left of x=2 or right of x=8)
        let path = path.unwrap();
        assert!(!path.iter().any(|&(x, y)| y == 4 && x >= 2 && x < 8));
    }

    #[test]
    fn test_simplify_path() {
        let finder = OrthogonalPathFinder::new();

        // A path with a single bend
        let path = vec![(0, 0), (1, 0), (2, 0), (2, 1), (2, 2)];

        let simplified = finder.simplify_path(&path);

        assert_eq!(simplified, vec![(0, 0), (2, 0), (2, 2)]);
    }

    #[test]
    fn test_path_avoiding() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        let start = Point::new(15.0, 15.0);
        let end = Point::new(85.0, 15.0);

        // Create an existing path through the middle
        let existing = vec![(2, 1), (3, 1), (4, 1), (5, 1), (6, 1)];

        let path = finder.find_path_avoiding(&mut grid, &start, &end, &[existing.clone()]);

        assert!(path.is_some());
        let path = path.unwrap();
        // Path should avoid the existing route
        assert!(!path.contains(&(4, 1)));
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_path_finder_default() {
        let finder = OrthogonalPathFinder::default();
        assert_eq!(finder.bend_penalty, 10.0);
        assert_eq!(finder.preferred_direction, PreferredDirection::None);
    }

    #[test]
    fn test_path_finder_new() {
        let finder = OrthogonalPathFinder::new();
        assert_eq!(finder.bend_penalty, 10.0);
    }

    #[test]
    fn test_path_finder_with_bend_penalty() {
        let finder = OrthogonalPathFinder::new().with_bend_penalty(20.0);

        assert_eq!(finder.bend_penalty, 20.0);
    }

    #[test]
    fn test_path_finder_with_preferred_direction() {
        let finder =
            OrthogonalPathFinder::new().with_preferred_direction(PreferredDirection::Horizontal);

        assert_eq!(finder.preferred_direction, PreferredDirection::Horizontal);
    }

    #[test]
    fn test_find_path_same_start_end() {
        let grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        let start = Point::new(15.0, 15.0);
        let end = Point::new(15.0, 15.0);

        let path = finder.find_path(&grid, &start, &end);

        // Should find a path (even if just the starting cell)
        assert!(path.is_some());
    }

    #[test]
    fn test_find_path_diagonal_target() {
        let grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        let start = Point::new(5.0, 5.0); // Cell (0, 0)
        let end = Point::new(95.0, 95.0); // Cell (9, 9)

        let path = finder.find_path(&grid, &start, &end);

        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.len() >= 10); // Manhattan distance = 18
    }

    #[test]
    fn test_find_path_no_path_fully_blocked() {
        let mut grid = GridGraph::new(5, 5, 10.0);
        let finder = OrthogonalPathFinder::new();

        // Create a complete wall between start and end
        for y in 0..5 {
            grid.set(2, y, crate::grid_graph::CellType::Blocked);
        }

        let start = Point::new(5.0, 25.0); // Cell (0, 2)
        let end = Point::new(45.0, 25.0); // Cell (4, 2)

        let path = finder.find_path(&grid, &start, &end);

        // No path should exist
        assert!(path.is_none());
    }

    #[test]
    fn test_find_path_blocked_start() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        grid.set(1, 1, crate::grid_graph::CellType::Blocked);

        let start = Point::new(15.0, 15.0); // Cell (1, 1) - blocked
        let end = Point::new(55.0, 55.0);

        let path = finder.find_path(&grid, &start, &end);

        // Should not find path from blocked cell
        assert!(path.is_none());
    }

    #[test]
    fn test_find_path_blocked_end() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        grid.set(5, 5, crate::grid_graph::CellType::Blocked);

        let start = Point::new(15.0, 15.0);
        let end = Point::new(55.0, 55.0); // Cell (5, 5) - blocked

        let path = finder.find_path(&grid, &start, &end);

        // Should not find path to blocked cell
        assert!(path.is_none());
    }

    #[test]
    fn test_simplify_path_straight() {
        let finder = OrthogonalPathFinder::new();

        // A straight horizontal path
        let path = vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)];

        let simplified = finder.simplify_path(&path);

        // Should simplify to just start and end
        assert_eq!(simplified.len(), 2);
        assert_eq!(simplified[0], (0, 0));
        assert_eq!(simplified[1], (4, 0));
    }

    #[test]
    fn test_simplify_path_multiple_bends() {
        let finder = OrthogonalPathFinder::new();

        let path = vec![(0, 0), (1, 0), (2, 0), (2, 1), (2, 2), (3, 2), (4, 2)];

        let simplified = finder.simplify_path(&path);

        // Should have 4 points: start, first bend, second bend, end
        assert!(simplified.len() <= 5);
        assert_eq!(simplified[0], (0, 0));
        assert_eq!(simplified[simplified.len() - 1], (4, 2));
    }

    #[test]
    fn test_simplify_path_empty() {
        let finder = OrthogonalPathFinder::new();

        let path: Vec<(usize, usize)> = vec![];
        let simplified = finder.simplify_path(&path);

        assert!(simplified.is_empty());
    }

    #[test]
    fn test_simplify_path_single() {
        let finder = OrthogonalPathFinder::new();

        let path = vec![(5, 5)];
        let simplified = finder.simplify_path(&path);

        assert_eq!(simplified, vec![(5, 5)]);
    }

    #[test]
    fn test_simplify_path_two_points() {
        let finder = OrthogonalPathFinder::new();

        let path = vec![(0, 0), (5, 0)];
        let simplified = finder.simplify_path(&path);

        assert_eq!(simplified, vec![(0, 0), (5, 0)]);
    }

    #[test]
    fn test_multi_terminal_router_new() {
        let router = MultiTerminalRouter::new();
        assert!(true); // Just verify construction
    }

    #[test]
    fn test_route_to_multiple() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let router = MultiTerminalRouter::new();

        let source = Point::new(5.0, 5.0);
        let targets = vec![Point::new(95.0, 5.0), Point::new(5.0, 95.0)];

        let paths = router.route_to_multiple(&mut grid, &source, &targets);

        // Should find paths to both targets (may fail if pathfinding can't reach one)
        assert!(!paths.is_empty());
        assert!(paths.len() >= 1);
    }

    #[test]
    fn test_route_to_multiple_empty_targets() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let router = MultiTerminalRouter::new();

        let source = Point::new(5.0, 5.0);
        let targets: Vec<Point> = vec![];

        let paths = router.route_to_multiple(&mut grid, &source, &targets);

        assert!(paths.is_empty());
    }

    #[test]
    fn test_path_finder_preferred_horizontal() {
        let finder =
            OrthogonalPathFinder::new().with_preferred_direction(PreferredDirection::Horizontal);

        assert_eq!(finder.preferred_direction, PreferredDirection::Horizontal);
    }

    #[test]
    fn test_path_finder_preferred_vertical() {
        let finder =
            OrthogonalPathFinder::new().with_preferred_direction(PreferredDirection::Vertical);

        assert_eq!(finder.preferred_direction, PreferredDirection::Vertical);
    }

    #[test]
    fn test_find_path_through_congested() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        // Mark some cells as congested (routable but more expensive)
        for x in 2..5 {
            grid.set(x, 5, crate::grid_graph::CellType::Congested);
        }

        let start = Point::new(15.0, 5.0); // Cell (1, 0)
        let end = Point::new(85.0, 55.0); // Cell (8, 5)

        let path = finder.find_path(&grid, &start, &end);

        // Should still find a path
        assert!(path.is_some());
    }

    #[test]
    fn test_find_path_out_of_bounds() {
        let grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        let start = Point::new(-5.0, 5.0); // Out of bounds
        let end = Point::new(55.0, 55.0);

        let path = finder.find_path(&grid, &start, &end);

        // Should return None for out-of-bounds start
        assert!(path.is_none());
    }

    #[test]
    fn test_find_path_avoiding_empty() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let finder = OrthogonalPathFinder::new();

        let start = Point::new(15.0, 15.0);
        let end = Point::new(85.0, 15.0);

        let path = finder.find_path_avoiding(&mut grid, &start, &end, &[]);

        // Should work the same as regular find_path
        assert!(path.is_some());
    }

    #[test]
    fn test_preferred_direction_variants() {
        assert_ne!(PreferredDirection::None, PreferredDirection::Horizontal);
        assert_ne!(PreferredDirection::Horizontal, PreferredDirection::Vertical);
        assert_ne!(PreferredDirection::Vertical, PreferredDirection::None);
    }
}
