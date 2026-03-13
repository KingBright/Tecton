use tecton_layout::layout_context::{LayoutNode, LayoutResult};
use tecton_routing::route::RouteSet;
use tecton_types::error::Result;

/// Backend trait for rendering diagrams
///
/// Implementations of this trait can render to different output formats
/// (SVG, PNG, Canvas, etc.)
pub trait Backend {
    /// Render a complete diagram
    fn render(&mut self, layout: &LayoutResult, routes: Option<&RouteSet>) -> Result<()>;

    /// Render a single node
    fn render_node(&mut self, node: &LayoutNode) -> Result<()>;

    /// Render routes (edges)
    fn render_routes(&mut self, routes: &RouteSet) -> Result<()>;

    /// Get the output as a string (if applicable)
    fn output(&self) -> Option<String>;

    /// Save the output to a file
    fn save(&self, path: &str) -> Result<()>;
}

/// Debug backend for visualizing the routing grid
pub struct DebugBackend {
    grid_cells: Vec<((usize, usize), char)>,
    width: usize,
    height: usize,
}

impl DebugBackend {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid_cells: Vec::new(),
            width,
            height,
        }
    }

    pub fn add_cell(&mut self, x: usize, y: usize, symbol: char) {
        self.grid_cells.push(((x, y), symbol));
    }

    pub fn render_grid(&self) -> String {
        let mut grid = vec![vec!['.'; self.width]; self.height];

        for ((x, y), symbol) in &self.grid_cells {
            if *x < self.width && *y < self.height {
                grid[*y][*x] = *symbol;
            }
        }

        grid.iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Backend for DebugBackend {
    fn render(&mut self, _layout: &LayoutResult, _routes: Option<&RouteSet>) -> Result<()> {
        Ok(())
    }

    fn render_node(&mut self, _node: &LayoutNode) -> Result<()> {
        Ok(())
    }

    fn render_routes(&mut self, _routes: &RouteSet) -> Result<()> {
        Ok(())
    }

    fn output(&self) -> Option<String> {
        Some(self.render_grid())
    }

    fn save(&self, _path: &str) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tecton_types::geometry::Rect;
    use tecton_types::id::NodeId;
    use tecton_types::style::Style;

    #[test]
    fn test_debug_backend_new() {
        let backend = DebugBackend::new(10, 10);
        assert!(backend.grid_cells.is_empty());
        assert_eq!(backend.width, 10);
        assert_eq!(backend.height, 10);
    }

    #[test]
    fn test_debug_backend_add_cell() {
        let mut backend = DebugBackend::new(10, 10);
        backend.add_cell(5, 5, 'X');

        assert_eq!(backend.grid_cells.len(), 1);
        assert_eq!(backend.grid_cells[0], ((5, 5), 'X'));
    }

    #[test]
    fn test_debug_backend_render_grid_empty() {
        let backend = DebugBackend::new(3, 2);
        let output = backend.render_grid();

        let expected = "...\n...";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_debug_backend_render_grid_with_cells() {
        let mut backend = DebugBackend::new(3, 3);
        backend.add_cell(0, 0, 'A');
        backend.add_cell(2, 2, 'B');

        let output = backend.render_grid();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with('A'));
        assert!(lines[2].ends_with('B'));
    }

    #[test]
    fn test_debug_backend_render_grid_out_of_bounds() {
        let mut backend = DebugBackend::new(3, 3);
        backend.add_cell(10, 10, 'X'); // Out of bounds

        let output = backend.render_grid();

        // Should still have a 3x3 grid of dots
        let expected = "...\n...\n...";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_debug_backend_render_contract() {
        let mut backend = DebugBackend::new(5, 5);

        let node = LayoutNode::new(
            NodeId::new(),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Style::default(),
        );
        let layout = LayoutResult::new(node, Rect::new(0.0, 0.0, 100.0, 100.0));

        // Test Backend trait methods
        let result = backend.render(&layout, None);
        assert!(result.is_ok());

        let result = backend.render_node(&LayoutNode::new(
            NodeId::new(),
            Rect::default(),
            Style::default(),
        ));
        assert!(result.is_ok());

        let result = backend.render_routes(&RouteSet::new());
        assert!(result.is_ok());

        let output = backend.output();
        assert!(output.is_some());

        let result = backend.save("/tmp/test.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_debug_backend_output_format() {
        let mut backend = DebugBackend::new(5, 5);
        backend.add_cell(0, 0, '#');
        backend.add_cell(4, 4, '@');

        let output = backend.output().unwrap();

        assert!(output.starts_with("#...."));
        assert!(output.ends_with("....@"));
        assert_eq!(output.matches('\n').count(), 4); // 5 rows = 4 newlines
    }
}
