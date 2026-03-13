use std::collections::HashMap;

use tecton_types::geometry::{Point, Rect};

/// Type of cell in the routing grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellType {
    /// Empty space available for routing
    Empty,
    /// Blocked by a node (cannot route through)
    Blocked,
    /// Reserved for edge routing
    Reserved,
    /// Currently occupied by a route
    Occupied,
    /// Congested area (high traffic)
    Congested,
}

impl CellType {
    /// Check if this cell can be used for routing
    pub fn is_routable(&self) -> bool {
        matches!(
            self,
            CellType::Empty | CellType::Reserved | CellType::Congested
        )
    }

    /// Check if this cell is blocked
    pub fn is_blocked(&self) -> bool {
        matches!(self, CellType::Blocked)
    }

    /// Get the routing cost for this cell
    pub fn cost(&self) -> f32 {
        match self {
            CellType::Empty => 1.0,
            CellType::Reserved => 2.0,
            CellType::Congested => 5.0,
            CellType::Occupied => 10.0,
            CellType::Blocked => f32::INFINITY,
        }
    }
}

impl Default for CellType {
    fn default() -> Self {
        CellType::Empty
    }
}

/// A grid-based graph for routing
///
/// This discretizes the canvas into a grid for efficient pathfinding
#[derive(Debug, Clone)]
pub struct GridGraph {
    /// Grid width in cells
    pub width: usize,
    /// Grid height in cells
    pub height: usize,
    /// Cell size (square cells)
    pub cell_size: f32,
    /// Grid cells
    cells: Vec<CellType>,
    /// Cell occupancy counts (for congestion tracking)
    occupancy: Vec<u32>,
}

impl GridGraph {
    /// Create a new grid graph
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self {
        let num_cells = width * height;
        Self {
            width,
            height,
            cell_size,
            cells: vec![CellType::Empty; num_cells],
            occupancy: vec![0; num_cells],
        }
    }

    /// Create a grid from a bounding rectangle
    pub fn from_rect(bounds: &Rect, cell_size: f32) -> Self {
        let width = (bounds.width() / cell_size).ceil() as usize;
        let height = (bounds.height() / cell_size).ceil() as usize;
        Self::new(width.max(1), height.max(1), cell_size)
    }

    /// Get the cell at grid coordinates
    pub fn get(&self, x: usize, y: usize) -> Option<CellType> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.cells.get(y * self.width + x).copied()
    }

    /// Set the cell at grid coordinates
    pub fn set(&mut self, x: usize, y: usize, cell_type: CellType) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = cell_type;
        }
    }

    /// Get cell index from point coordinates
    pub fn point_to_cell(&self, point: &Point) -> Option<(usize, usize)> {
        let x = (point.x() / self.cell_size).floor() as isize;
        let y = (point.y() / self.cell_size).floor() as isize;

        if x < 0 || y < 0 {
            return None;
        }

        let x = x as usize;
        let y = y as usize;

        if x >= self.width || y >= self.height {
            return None;
        }

        Some((x, y))
    }

    /// Get point coordinates from cell index
    pub fn cell_to_point(&self, x: usize, y: usize) -> Option<Point> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(Point::new(
            x as f32 * self.cell_size + self.cell_size / 2.0,
            y as f32 * self.cell_size + self.cell_size / 2.0,
        ))
    }

    /// Get the cell containing a point
    pub fn cell_at_point(&self, point: &Point) -> Option<CellType> {
        self.point_to_cell(point).and_then(|(x, y)| self.get(x, y))
    }

    /// Block all cells intersecting a rectangle
    pub fn block_rect(&mut self, rect: &Rect) {
        let min_x = (rect.left() / self.cell_size).floor() as usize;
        let max_x = (rect.right() / self.cell_size).ceil() as usize;
        let min_y = (rect.top() / self.cell_size).floor() as usize;
        let max_y = (rect.bottom() / self.cell_size).ceil() as usize;

        for x in min_x..=max_x.min(self.width.saturating_sub(1)) {
            for y in min_y..=max_y.min(self.height.saturating_sub(1)) {
                self.set(x, y, CellType::Blocked);
            }
        }
    }

    /// Block all cells intersecting a rectangle, but leave a margin around the edges
    /// This is useful for leaving space for ports on the edges of nodes
    pub fn block_rect_with_margin(&mut self, rect: &Rect, margin: f32) {
        // Shrink the rectangle by the margin to leave edge cells unblocked
        let margin_rect = Rect::new(
            rect.left() + margin,
            rect.top() + margin,
            (rect.width() - 2.0 * margin).max(0.0),
            (rect.height() - 2.0 * margin).max(0.0),
        );

        // Only block if the margin rect is valid
        if margin_rect.width() > 0.0 && margin_rect.height() > 0.0 {
            // Use floor for max coordinates to ensure edge cells are NOT blocked
            // This is critical for ports that lie exactly on the edge of the node
            let min_x = (margin_rect.left() / self.cell_size).floor() as usize;
            let max_x = (margin_rect.right() / self.cell_size).floor() as usize;
            let min_y = (margin_rect.top() / self.cell_size).floor() as usize;
            let max_y = (margin_rect.bottom() / self.cell_size).floor() as usize;

            for x in min_x..=max_x.min(self.width.saturating_sub(1)) {
                for y in min_y..=max_y.min(self.height.saturating_sub(1)) {
                    self.set(x, y, CellType::Blocked);
                }
            }
        }
    }

    /// Reserve cells for routing
    pub fn reserve_rect(&mut self, rect: &Rect) {
        let min_x = (rect.left() / self.cell_size).floor() as usize;
        let max_x = (rect.right() / self.cell_size).ceil() as usize;
        let min_y = (rect.top() / self.cell_size).floor() as usize;
        let max_y = (rect.bottom() / self.cell_size).ceil() as usize;

        for x in min_x..=max_x.min(self.width.saturating_sub(1)) {
            for y in min_y..=max_y.min(self.height.saturating_sub(1)) {
                if self.get(x, y) == Some(CellType::Empty) {
                    self.set(x, y, CellType::Reserved);
                }
            }
        }
    }

    /// Mark a cell as occupied by a route
    pub fn mark_occupied(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.cells[idx] = CellType::Occupied;
            self.occupancy[idx] += 1;
        }
    }

    /// Increment occupancy for a cell (for congestion tracking)
    pub fn increment_occupancy(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.occupancy[idx] += 1;

            // Mark as congested if occupancy exceeds threshold
            if self.occupancy[idx] > 3 {
                self.cells[idx] = CellType::Congested;
            }
        }
    }

    /// Get occupancy count for a cell
    pub fn get_occupancy(&self, x: usize, y: usize) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        self.occupancy[y * self.width + x]
    }

    /// Get neighbors of a cell (for pathfinding)
    pub fn neighbors(&self, x: usize, y: usize) -> Vec<((usize, usize), f32)> {
        let mut result = Vec::new();

        // Orthogonal neighbors only (for Manhattan routing)
        let deltas = [(0, -1), (0, 1), (-1, 0), (1, 0)];

        for (dx, dy) in deltas {
            let nx = x as isize + dx;
            let ny = y as isize + dy;

            if nx >= 0 && ny >= 0 {
                let nx = nx as usize;
                let ny = ny as usize;

                if let Some(cell) = self.get(nx, ny) {
                    if cell.is_routable() {
                        result.push(((nx, ny), cell.cost()));
                    }
                }
            }
        }

        result
    }

    /// Clear all routes (reset Occupied cells to Empty)
    pub fn clear_routes(&mut self) {
        for cell in &mut self.cells {
            if *cell == CellType::Occupied {
                *cell = CellType::Empty;
            }
        }
    }

    /// Clear all congestion
    pub fn clear_congestion(&mut self) {
        for (i, cell) in self.cells.iter_mut().enumerate() {
            if *cell == CellType::Congested {
                *cell = CellType::Empty;
            }
            self.occupancy[i] = 0;
        }
    }

    /// Get all congested cells
    pub fn congested_cells(&self) -> Vec<(usize, usize, u32)> {
        let mut result = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                if self.cells[idx] == CellType::Congested || self.occupancy[idx] > 2 {
                    result.push((x, y, self.occupancy[idx]));
                }
            }
        }
        result
    }

    /// Get the bounds of the grid as a rectangle
    pub fn bounds(&self) -> Rect {
        Rect::new(
            0.0,
            0.0,
            self.width as f32 * self.cell_size,
            self.height as f32 * self.cell_size,
        )
    }
}

/// A path through the grid
#[derive(Debug, Clone, PartialEq)]
pub struct GridPath {
    pub cells: Vec<(usize, usize)>,
    pub cost: f32,
}

impl GridPath {
    pub fn new(cells: Vec<(usize, usize)>, cost: f32) -> Self {
        Self { cells, cost }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Convert grid cells to points
    pub fn to_points(&self, grid: &GridGraph) -> Vec<Point> {
        self.cells
            .iter()
            .filter_map(|(x, y)| grid.cell_to_point(*x, *y))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let grid = GridGraph::new(10, 10, 10.0);
        assert_eq!(grid.width, 10);
        assert_eq!(grid.height, 10);
        assert_eq!(grid.cell_size, 10.0);
    }

    #[test]
    fn test_point_to_cell() {
        let grid = GridGraph::new(10, 10, 10.0);

        assert_eq!(grid.point_to_cell(&Point::new(5.0, 5.0)), Some((0, 0)));
        assert_eq!(grid.point_to_cell(&Point::new(15.0, 25.0)), Some((1, 2)));
        assert_eq!(grid.point_to_cell(&Point::new(105.0, 5.0)), None); // Out of bounds
    }

    #[test]
    fn test_block_rect() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let rect = Rect::new(15.0, 15.0, 20.0, 20.0);

        grid.block_rect(&rect);

        assert_eq!(grid.get(1, 1), Some(CellType::Blocked));
        assert_eq!(grid.get(3, 3), Some(CellType::Blocked));
        assert_eq!(grid.get(0, 0), Some(CellType::Empty));
    }

    #[test]
    fn test_neighbors() {
        let grid = GridGraph::new(5, 5, 10.0);
        let neighbors = grid.neighbors(2, 2);

        assert_eq!(neighbors.len(), 4);
        assert!(neighbors.contains(&((2, 1), 1.0)));
        assert!(neighbors.contains(&((2, 3), 1.0)));
        assert!(neighbors.contains(&((1, 2), 1.0)));
        assert!(neighbors.contains(&((3, 2), 1.0)));
    }

    #[test]
    fn test_occupancy() {
        let mut grid = GridGraph::new(5, 5, 10.0);

        grid.increment_occupancy(2, 2);
        grid.increment_occupancy(2, 2);
        grid.increment_occupancy(2, 2);
        grid.increment_occupancy(2, 2);

        assert_eq!(grid.get_occupancy(2, 2), 4);
        assert_eq!(grid.get(2, 2), Some(CellType::Congested));
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_grid_from_rect() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        let grid = GridGraph::from_rect(&bounds, 10.0);

        assert_eq!(grid.width, 10);
        assert_eq!(grid.height, 5);
        assert_eq!(grid.cell_size, 10.0);
    }

    #[test]
    fn test_grid_from_rect_small() {
        let bounds = Rect::new(0.0, 0.0, 5.0, 3.0);
        let grid = GridGraph::from_rect(&bounds, 10.0);

        // Should have at least 1 cell in each dimension
        assert_eq!(grid.width, 1);
        assert_eq!(grid.height, 1);
    }

    #[test]
    fn test_grid_get_out_of_bounds() {
        let grid = GridGraph::new(5, 5, 10.0);

        assert_eq!(grid.get(10, 0), None);
        assert_eq!(grid.get(0, 10), None);
    }

    #[test]
    fn test_grid_set_out_of_bounds() {
        let mut grid = GridGraph::new(5, 5, 10.0);

        // Should not panic
        grid.set(10, 0, CellType::Blocked);
        grid.set(0, 10, CellType::Blocked);

        assert_eq!(grid.get(0, 0), Some(CellType::Empty));
    }

    #[test]
    fn test_cell_to_point() {
        let grid = GridGraph::new(10, 10, 10.0);

        let point = grid.cell_to_point(0, 0);
        assert_eq!(point, Some(Point::new(5.0, 5.0))); // Center of cell

        let point = grid.cell_to_point(1, 2);
        assert_eq!(point, Some(Point::new(15.0, 25.0)));

        let point = grid.cell_to_point(10, 0);
        assert_eq!(point, None); // Out of bounds
    }

    #[test]
    fn test_cell_at_point() {
        let grid = GridGraph::new(10, 10, 10.0);

        assert_eq!(
            grid.cell_at_point(&Point::new(5.0, 5.0)),
            Some(CellType::Empty)
        );
        assert_eq!(
            grid.cell_at_point(&Point::new(15.0, 25.0)),
            Some(CellType::Empty)
        );
    }

    #[test]
    fn test_reserve_rect() {
        let mut grid = GridGraph::new(10, 10, 10.0);
        let rect = Rect::new(15.0, 15.0, 20.0, 20.0);

        grid.reserve_rect(&rect);

        assert_eq!(grid.get(1, 1), Some(CellType::Reserved));
        assert_eq!(grid.get(0, 0), Some(CellType::Empty));
    }

    #[test]
    fn test_mark_occupied() {
        let mut grid = GridGraph::new(5, 5, 10.0);

        grid.mark_occupied(2, 2);

        assert_eq!(grid.get(2, 2), Some(CellType::Occupied));
        assert_eq!(grid.get_occupancy(2, 2), 1);
    }

    #[test]
    fn test_increment_occupancy_threshold() {
        let mut grid = GridGraph::new(5, 5, 10.0);

        // Threshold for congestion is > 3
        for _ in 0..3 {
            grid.increment_occupancy(2, 2);
        }
        assert_eq!(grid.get(2, 2), Some(CellType::Empty));

        grid.increment_occupancy(2, 2); // 4th increment
        assert_eq!(grid.get(2, 2), Some(CellType::Congested));
    }

    #[test]
    fn test_clear_routes() {
        let mut grid = GridGraph::new(5, 5, 10.0);

        grid.mark_occupied(2, 2);
        grid.mark_occupied(3, 3);
        grid.set(0, 0, CellType::Blocked);

        grid.clear_routes();

        assert_eq!(grid.get(2, 2), Some(CellType::Empty));
        assert_eq!(grid.get(3, 3), Some(CellType::Empty));
        assert_eq!(grid.get(0, 0), Some(CellType::Blocked)); // Blocked should remain
    }

    #[test]
    fn test_clear_congestion() {
        let mut grid = GridGraph::new(5, 5, 10.0);

        // Create congestion
        for _ in 0..5 {
            grid.increment_occupancy(2, 2);
        }

        grid.clear_congestion();

        assert_eq!(grid.get(2, 2), Some(CellType::Empty));
        assert_eq!(grid.get_occupancy(2, 2), 0);
    }

    #[test]
    fn test_congested_cells() {
        let mut grid = GridGraph::new(5, 5, 10.0);

        // Create congestion at (2, 2) and (3, 3)
        for _ in 0..5 {
            grid.increment_occupancy(2, 2);
            grid.increment_occupancy(3, 3);
        }

        let congested = grid.congested_cells();

        assert_eq!(congested.len(), 2);
    }

    #[test]
    fn test_grid_bounds() {
        let grid = GridGraph::new(10, 5, 10.0);
        let bounds = grid.bounds();

        assert_eq!(bounds.width(), 100.0);
        assert_eq!(bounds.height(), 50.0);
    }

    #[test]
    fn test_neighbors_at_edge() {
        let grid = GridGraph::new(5, 5, 10.0);

        // Corner cell should have only 2 neighbors
        let neighbors = grid.neighbors(0, 0);
        assert_eq!(neighbors.len(), 2);

        // Edge cell should have 3 neighbors
        let neighbors = grid.neighbors(0, 2);
        assert_eq!(neighbors.len(), 3);
    }

    #[test]
    fn test_neighbors_blocked_cell() {
        let mut grid = GridGraph::new(5, 5, 10.0);
        grid.set(2, 1, CellType::Blocked);
        grid.set(1, 2, CellType::Blocked);

        let neighbors = grid.neighbors(2, 2);

        // Should only have 2 routable neighbors (down and right)
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_cell_type_is_routable() {
        assert!(CellType::Empty.is_routable());
        assert!(CellType::Reserved.is_routable());
        assert!(CellType::Congested.is_routable());
        assert!(!CellType::Blocked.is_routable());
        assert!(!CellType::Occupied.is_routable());
    }

    #[test]
    fn test_cell_type_is_blocked() {
        assert!(CellType::Blocked.is_blocked());
        assert!(!CellType::Empty.is_blocked());
    }

    #[test]
    fn test_cell_type_cost() {
        assert_eq!(CellType::Empty.cost(), 1.0);
        assert_eq!(CellType::Reserved.cost(), 2.0);
        assert_eq!(CellType::Congested.cost(), 5.0);
        assert_eq!(CellType::Occupied.cost(), 10.0);
        assert!(CellType::Blocked.cost().is_infinite());
    }

    #[test]
    fn test_cell_type_default() {
        assert_eq!(CellType::default(), CellType::Empty);
    }

    #[test]
    fn test_grid_path_new() {
        let path = GridPath::new(vec![(0, 0), (1, 0), (1, 1)], 2.0);

        assert_eq!(path.len(), 3);
        assert!(!path.is_empty());
        assert_eq!(path.cost, 2.0);
    }

    #[test]
    fn test_grid_path_empty() {
        let path = GridPath::new(vec![], 0.0);

        assert_eq!(path.len(), 0);
        assert!(path.is_empty());
    }

    #[test]
    fn test_grid_path_to_points() {
        let grid = GridGraph::new(10, 10, 10.0);
        let path = GridPath::new(vec![(0, 0), (1, 0)], 1.0);

        let points = path.to_points(&grid);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0], Point::new(5.0, 5.0));
        assert_eq!(points[1], Point::new(15.0, 5.0));
    }

    #[test]
    fn test_point_to_cell_negative_coords() {
        let grid = GridGraph::new(10, 10, 10.0);

        // Negative coordinates should return None
        assert_eq!(grid.point_to_cell(&Point::new(-5.0, 5.0)), None);
        assert_eq!(grid.point_to_cell(&Point::new(5.0, -5.0)), None);
    }

    #[test]
    fn test_large_grid() {
        // Test creating a large grid
        let grid = GridGraph::new(1000, 1000, 1.0);

        assert_eq!(grid.width, 1000);
        assert_eq!(grid.height, 1000);

        // Should be able to access cells
        assert_eq!(grid.get(500, 500), Some(CellType::Empty));
    }

    #[test]
    fn test_boundary_point_conversion() {
        let grid = GridGraph::new(10, 10, 10.0);

        // Point exactly on boundary
        assert_eq!(grid.point_to_cell(&Point::new(100.0, 5.0)), None); // Out of bounds

        // Point just inside boundary
        assert_eq!(grid.point_to_cell(&Point::new(99.9, 5.0)), Some((9, 0)));
    }
}
