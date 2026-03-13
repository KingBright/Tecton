use tecton_types::{
    edge::{EdgeKind, RouteSegment},
    geometry::Point,
    id::{EdgeId, PortId},
    style::Color,
};

/// A complete route for an edge
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// The edge this route is for
    pub edge_id: EdgeId,
    /// Source port
    pub source_port: PortId,
    /// Target port
    pub target_port: PortId,
    /// The path as a series of segments
    pub segments: Vec<RouteSegment>,
    /// Total length of the route
    pub length: f32,
    /// Number of bends (direction changes)
    pub bend_count: usize,
    /// Whether this route crosses any congested areas
    pub crosses_congestion: bool,
    /// Optional label for the edge
    pub label: Option<String>,
    /// The kind of edge (affects stroke style)
    pub edge_kind: EdgeKind,
    /// The color of the edge
    pub edge_color: Color,
}

impl Route {
    pub fn new(edge_id: EdgeId, source_port: PortId, target_port: PortId) -> Self {
        Self {
            edge_id,
            source_port,
            target_port,
            segments: Vec::new(),
            length: 0.0,
            bend_count: 0,
            crosses_congestion: false,
            label: None,
            edge_kind: EdgeKind::default(),
            edge_color: Color::GRAY_500,
        }
    }

    pub fn from_segments(
        edge_id: EdgeId,
        source_port: PortId,
        target_port: PortId,
        segments: Vec<RouteSegment>,
    ) -> Self {
        let length = Self::calculate_length(&segments);
        let bend_count = Self::count_bends(&segments);

        Self {
            edge_id,
            source_port,
            target_port,
            segments,
            length,
            bend_count,
            crosses_congestion: false,
            label: None,
            edge_kind: EdgeKind::default(),
            edge_color: Color::GRAY_500,
        }
    }

    pub fn with_congestion(mut self, congested: bool) -> Self {
        self.crosses_congestion = congested;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_edge_kind(mut self, kind: EdgeKind) -> Self {
        self.edge_kind = kind;
        self
    }

    pub fn with_edge_color(mut self, color: Color) -> Self {
        self.edge_color = color;
        self
    }

    /// Add a segment to the route
    pub fn add_segment(&mut self, segment: RouteSegment) {
        self.segments.push(segment);
        self.recalculate_stats();
    }

    /// Get the start point of the route
    pub fn start_point(&self) -> Option<Point> {
        self.segments.first().map(|s| Point::new(s.x, s.y))
    }

    /// Get the end point of the route
    pub fn end_point(&self) -> Option<Point> {
        self.segments.last().map(|s| Point::new(s.x, s.y))
    }

    /// Get all points in the route
    pub fn points(&self) -> Vec<Point> {
        self.segments.iter().map(|s| Point::new(s.x, s.y)).collect()
    }

    /// Convert grid path to route segments
    pub fn from_grid_path(
        edge_id: EdgeId,
        source_port: PortId,
        target_port: PortId,
        grid_path: &[(usize, usize)],
        cell_size: f32,
        source_pos: Point,
        target_pos: Point,
    ) -> Self {
        let mut segments = Vec::new();

        segments.push(RouteSegment::move_to(source_pos.x(), source_pos.y()));

        if grid_path.is_empty() {
            segments.push(RouteSegment::line_to(target_pos.x(), target_pos.y()));
            let mut route = Self::from_segments(edge_id, source_port, target_port, segments)
                .with_edge_kind(EdgeKind::default())
                .with_edge_color(Color::GRAY_500);
            route.simplify();
            return route;
        }

        let first_x = grid_path[0].0 as f32 * cell_size + cell_size / 2.0;
        let first_y = grid_path[0].1 as f32 * cell_size + cell_size / 2.0;

        if (source_pos.x() - first_x).abs() > (source_pos.y() - first_y).abs() {
            segments.push(RouteSegment::line_to(first_x, source_pos.y()));
        } else {
            segments.push(RouteSegment::line_to(source_pos.x(), first_y));
        }

        for &(x, y) in grid_path {
            let px = x as f32 * cell_size + cell_size / 2.0;
            let py = y as f32 * cell_size + cell_size / 2.0;
            segments.push(RouteSegment::line_to(px, py));
        }

        let last_x = grid_path.last().unwrap().0 as f32 * cell_size + cell_size / 2.0;
        let last_y = grid_path.last().unwrap().1 as f32 * cell_size + cell_size / 2.0;

        if (target_pos.x() - last_x).abs() > (target_pos.y() - last_y).abs() {
            segments.push(RouteSegment::line_to(target_pos.x(), last_y));
        } else {
            segments.push(RouteSegment::line_to(last_x, target_pos.y()));
        }

        segments.push(RouteSegment::line_to(target_pos.x(), target_pos.y()));

        let mut route = Self::from_segments(edge_id, source_port, target_port, segments)
            .with_edge_kind(EdgeKind::default())
            .with_edge_color(Color::GRAY_500);
            
        route.simplify();
        route
    }

    fn recalculate_stats(&mut self) {
        self.length = Self::calculate_length(&self.segments);
        self.bend_count = Self::count_bends(&self.segments);
    }

    fn calculate_length(segments: &[RouteSegment]) -> f32 {
        if segments.len() < 2 {
            return 0.0;
        }

        let mut total = 0.0;
        for i in 1..segments.len() {
            let dx = segments[i].x - segments[i - 1].x;
            let dy = segments[i].y - segments[i - 1].y;
            total += (dx * dx + dy * dy).sqrt();
        }
        total
    }

    fn count_bends(segments: &[RouteSegment]) -> usize {
        if segments.len() < 3 {
            return 0;
        }

        let mut bends = 0;
        for i in 2..segments.len() {
            let dx1 = segments[i - 1].x - segments[i - 2].x;
            let dy1 = segments[i - 1].y - segments[i - 2].y;
            let dx2 = segments[i].x - segments[i - 1].x;
            let dy2 = segments[i].y - segments[i - 1].y;

            // Check if direction changed (cross product non-zero for non-parallel)
            if dx1 * dy2 != dx2 * dy1 {
                bends += 1;
            }
        }
        bends
    }

    /// Simplify the route by removing collinear points
    pub fn simplify(&mut self) {
        if self.segments.len() <= 2 {
            return;
        }

        let mut simplified = vec![self.segments[0].clone()];

        for i in 1..self.segments.len() - 1 {
            let prev = &simplified[simplified.len() - 1];
            let curr = &self.segments[i];
            let next = &self.segments[i + 1];

            let dx1 = curr.x - prev.x;
            let dy1 = curr.y - prev.y;
            let dx2 = next.x - curr.x;
            let dy2 = next.y - curr.y;

            // Check if points are collinear (same direction)
            let cross = dx1 * dy2 - dy1 * dx2;
            if cross.abs() > f32::EPSILON {
                simplified.push(curr.clone());
            }
        }

        simplified.push(self.segments.last().unwrap().clone());
        self.segments = simplified;
        self.recalculate_stats();
    }
}

/// Collection of routes
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RouteSet {
    routes: Vec<Route>,
}

impl RouteSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            routes: Vec::with_capacity(capacity),
        }
    }

    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    pub fn get_route(&self, edge_id: EdgeId) -> Option<&Route> {
        self.routes.iter().find(|r| r.edge_id == edge_id)
    }

    pub fn get_route_mut(&mut self, edge_id: EdgeId) -> Option<&mut Route> {
        self.routes.iter_mut().find(|r| r.edge_id == edge_id)
    }

    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    pub fn routes_mut(&mut self) -> &mut Vec<Route> {
        &mut self.routes
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Get total length of all routes
    pub fn total_length(&self) -> f32 {
        self.routes.iter().map(|r| r.length).sum()
    }

    /// Get total bend count across all routes
    pub fn total_bends(&self) -> usize {
        self.routes.iter().map(|r| r.bend_count).sum()
    }

    /// Get routes that cross congested areas
    pub fn congested_routes(&self) -> Vec<&Route> {
        self.routes
            .iter()
            .filter(|r| r.crosses_congestion)
            .collect()
    }

    /// Simplify all routes
    pub fn simplify_all(&mut self) {
        for route in &mut self.routes {
            route.simplify();
        }
    }

    /// Clear all routes
    pub fn clear(&mut self) {
        self.routes.clear();
    }
}

/// Statistics about a set of routes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteStats {
    pub route_count: usize,
    pub total_length: f32,
    pub total_bends: usize,
    pub avg_length: f32,
    pub avg_bends: f32,
    pub max_length: f32,
    pub max_bends: usize,
    pub congested_count: usize,
}

impl RouteStats {
    pub fn from_routes(routes: &[Route]) -> Self {
        if routes.is_empty() {
            return Self {
                route_count: 0,
                total_length: 0.0,
                total_bends: 0,
                avg_length: 0.0,
                avg_bends: 0.0,
                max_length: 0.0,
                max_bends: 0,
                congested_count: 0,
            };
        }

        let route_count = routes.len();
        let total_length: f32 = routes.iter().map(|r| r.length).sum();
        let total_bends: usize = routes.iter().map(|r| r.bend_count).sum();
        let congested_count = routes.iter().filter(|r| r.crosses_congestion).count();

        let max_length = routes.iter().map(|r| r.length).fold(0.0, f32::max);
        let max_bends = routes.iter().map(|r| r.bend_count).max().unwrap_or(0);

        Self {
            route_count,
            total_length,
            total_bends,
            avg_length: total_length / route_count as f32,
            avg_bends: total_bends as f32 / route_count as f32,
            max_length,
            max_bends,
            congested_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_from_segments() {
        let segments = vec![
            RouteSegment::move_to(0.0, 0.0),
            RouteSegment::line_to(10.0, 0.0),
            RouteSegment::line_to(10.0, 10.0),
        ];

        let route = Route::from_segments(EdgeId::new(), PortId::new(), PortId::new(), segments);

        assert_eq!(route.length, 20.0);
        assert_eq!(route.bend_count, 1);
    }

    #[test]
    fn test_route_simplify() {
        let segments = vec![
            RouteSegment::move_to(0.0, 0.0),
            RouteSegment::line_to(5.0, 0.0),
            RouteSegment::line_to(10.0, 0.0),
            RouteSegment::line_to(10.0, 10.0),
        ];

        let mut route = Route::from_segments(EdgeId::new(), PortId::new(), PortId::new(), segments);
        route.simplify();

        assert_eq!(route.segments.len(), 3);
        assert_eq!(route.bend_count, 1);
    }

    #[test]
    fn test_route_stats() {
        let routes = vec![
            Route::from_segments(
                EdgeId::new(),
                PortId::new(),
                PortId::new(),
                vec![
                    RouteSegment::move_to(0.0, 0.0),
                    RouteSegment::line_to(10.0, 0.0),
                ],
            ),
            Route::from_segments(
                EdgeId::new(),
                PortId::new(),
                PortId::new(),
                vec![
                    RouteSegment::move_to(0.0, 0.0),
                    RouteSegment::line_to(20.0, 0.0),
                ],
            ),
        ];

        let stats = RouteStats::from_routes(&routes);

        assert_eq!(stats.route_count, 2);
        assert_eq!(stats.total_length, 30.0);
        assert_eq!(stats.avg_length, 15.0);
        assert_eq!(stats.max_length, 20.0);
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_route_new() {
        let edge_id = EdgeId::new();
        let source_port = PortId::new();
        let target_port = PortId::new();

        let route = Route::new(edge_id, source_port, target_port);

        assert_eq!(route.edge_id, edge_id);
        assert_eq!(route.source_port, source_port);
        assert_eq!(route.target_port, target_port);
        assert!(route.segments.is_empty());
        assert_eq!(route.length, 0.0);
        assert_eq!(route.bend_count, 0);
        assert!(!route.crosses_congestion);
    }

    #[test]
    fn test_route_with_congestion() {
        let route = Route::new(EdgeId::new(), PortId::new(), PortId::new()).with_congestion(true);

        assert!(route.crosses_congestion);
    }

    #[test]
    fn test_route_add_segment() {
        let mut route = Route::new(EdgeId::new(), PortId::new(), PortId::new());

        route.add_segment(RouteSegment::move_to(0.0, 0.0));
        route.add_segment(RouteSegment::line_to(10.0, 0.0));

        assert_eq!(route.segments.len(), 2);
        assert_eq!(route.length, 10.0);
    }

    #[test]
    fn test_route_start_point() {
        let segments = vec![
            RouteSegment::move_to(5.0, 10.0),
            RouteSegment::line_to(15.0, 10.0),
        ];

        let route = Route::from_segments(EdgeId::new(), PortId::new(), PortId::new(), segments);

        assert_eq!(route.start_point(), Some(Point::new(5.0, 10.0)));
    }

    #[test]
    fn test_route_end_point() {
        let segments = vec![
            RouteSegment::move_to(5.0, 10.0),
            RouteSegment::line_to(15.0, 20.0),
        ];

        let route = Route::from_segments(EdgeId::new(), PortId::new(), PortId::new(), segments);

        assert_eq!(route.end_point(), Some(Point::new(15.0, 20.0)));
    }

    #[test]
    fn test_route_start_point_empty() {
        let route = Route::new(EdgeId::new(), PortId::new(), PortId::new());

        assert_eq!(route.start_point(), None);
    }

    #[test]
    fn test_route_end_point_empty() {
        let route = Route::new(EdgeId::new(), PortId::new(), PortId::new());

        assert_eq!(route.end_point(), None);
    }

    #[test]
    fn test_route_points() {
        let segments = vec![
            RouteSegment::move_to(0.0, 0.0),
            RouteSegment::line_to(10.0, 0.0),
            RouteSegment::line_to(10.0, 10.0),
        ];

        let route = Route::from_segments(EdgeId::new(), PortId::new(), PortId::new(), segments);

        let points = route.points();

        assert_eq!(points.len(), 3);
        assert_eq!(points[0], Point::new(0.0, 0.0));
        assert_eq!(points[2], Point::new(10.0, 10.0));
    }

    #[test]
    fn test_route_from_grid_path() {
        let grid_path = vec![(0, 0), (1, 0), (1, 1)];

        let route = Route::from_grid_path(
            EdgeId::new(),
            PortId::new(),
            PortId::new(),
            &grid_path,
            10.0,
            Point::new(5.0, 5.0),
            Point::new(15.0, 15.0),
        );

        assert_eq!(route.segments.len(), 3);
        assert_eq!(route.segments[0].x, 5.0); // cell center
        assert_eq!(route.segments[0].y, 5.0);
    }

    #[test]
    fn test_route_empty() {
        let route = Route::new(EdgeId::new(), PortId::new(), PortId::new());

        assert_eq!(route.length, 0.0);
        assert_eq!(route.bend_count, 0);
    }

    #[test]
    fn test_route_single_segment() {
        let segments = vec![RouteSegment::move_to(0.0, 0.0)];

        let route = Route::from_segments(EdgeId::new(), PortId::new(), PortId::new(), segments);

        assert_eq!(route.length, 0.0);
        assert_eq!(route.bend_count, 0);
    }

    #[test]
    fn test_route_simplify_empty() {
        let mut route = Route::new(EdgeId::new(), PortId::new(), PortId::new());
        route.simplify();

        assert!(route.segments.is_empty());
    }

    #[test]
    fn test_route_simplify_two_points() {
        let segments = vec![
            RouteSegment::move_to(0.0, 0.0),
            RouteSegment::line_to(10.0, 0.0),
        ];

        let mut route = Route::from_segments(EdgeId::new(), PortId::new(), PortId::new(), segments);
        route.simplify();

        assert_eq!(route.segments.len(), 2);
    }

    #[test]
    fn test_route_simplify_complex() {
        let segments = vec![
            RouteSegment::move_to(0.0, 0.0),
            RouteSegment::line_to(5.0, 0.0),
            RouteSegment::line_to(10.0, 0.0),
            RouteSegment::line_to(10.0, 5.0),
            RouteSegment::line_to(10.0, 10.0),
        ];

        let mut route = Route::from_segments(EdgeId::new(), PortId::new(), PortId::new(), segments);
        route.simplify();

        // Should simplify to: (0,0), (10,0), (10,10)
        assert_eq!(route.segments.len(), 3);
    }

    #[test]
    fn test_route_set_new() {
        let route_set = RouteSet::new();

        assert!(route_set.is_empty());
        assert_eq!(route_set.len(), 0);
    }

    #[test]
    fn test_route_set_default() {
        let route_set = RouteSet::default();

        assert!(route_set.is_empty());
    }

    #[test]
    fn test_route_set_with_capacity() {
        let route_set = RouteSet::with_capacity(10);

        assert!(route_set.is_empty());
    }

    #[test]
    fn test_route_set_add_route() {
        let mut route_set = RouteSet::new();
        let route = Route::new(EdgeId::new(), PortId::new(), PortId::new());

        route_set.add_route(route);

        assert_eq!(route_set.len(), 1);
    }

    #[test]
    fn test_route_set_get_route() {
        let mut route_set = RouteSet::new();
        let edge_id = EdgeId::new();
        let route = Route::new(edge_id, PortId::new(), PortId::new());

        route_set.add_route(route);

        let found = route_set.get_route(edge_id);
        assert!(found.is_some());

        let not_found = route_set.get_route(EdgeId::new());
        assert!(not_found.is_none());
    }

    #[test]
    fn test_route_set_get_route_mut() {
        let mut route_set = RouteSet::new();
        let edge_id = EdgeId::new();
        let route = Route::new(edge_id, PortId::new(), PortId::new());

        route_set.add_route(route);

        let found = route_set.get_route_mut(edge_id);
        assert!(found.is_some());
        found.unwrap().crosses_congestion = true;

        let found = route_set.get_route(edge_id);
        assert!(found.unwrap().crosses_congestion);
    }

    #[test]
    fn test_route_set_routes() {
        let mut route_set = RouteSet::new();
        route_set.add_route(Route::new(EdgeId::new(), PortId::new(), PortId::new()));
        route_set.add_route(Route::new(EdgeId::new(), PortId::new(), PortId::new()));

        let routes = route_set.routes();

        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn test_route_set_total_length() {
        let mut route_set = RouteSet::new();

        let route1 = Route::from_segments(
            EdgeId::new(),
            PortId::new(),
            PortId::new(),
            vec![
                RouteSegment::move_to(0.0, 0.0),
                RouteSegment::line_to(10.0, 0.0),
            ],
        );

        let route2 = Route::from_segments(
            EdgeId::new(),
            PortId::new(),
            PortId::new(),
            vec![
                RouteSegment::move_to(0.0, 0.0),
                RouteSegment::line_to(20.0, 0.0),
            ],
        );

        route_set.add_route(route1);
        route_set.add_route(route2);

        assert_eq!(route_set.total_length(), 30.0);
    }

    #[test]
    fn test_route_set_total_bends() {
        let mut route_set = RouteSet::new();

        let route1 = Route::from_segments(
            EdgeId::new(),
            PortId::new(),
            PortId::new(),
            vec![
                RouteSegment::move_to(0.0, 0.0),
                RouteSegment::line_to(10.0, 0.0),
                RouteSegment::line_to(10.0, 10.0),
            ],
        );

        let route2 = Route::from_segments(
            EdgeId::new(),
            PortId::new(),
            PortId::new(),
            vec![
                RouteSegment::move_to(0.0, 0.0),
                RouteSegment::line_to(10.0, 0.0),
                RouteSegment::line_to(10.0, 10.0),
                RouteSegment::line_to(20.0, 10.0),
            ],
        );

        route_set.add_route(route1);
        route_set.add_route(route2);

        assert_eq!(route_set.total_bends(), 3); // 1 + 2 bends
    }

    #[test]
    fn test_route_set_congested_routes() {
        let mut route_set = RouteSet::new();

        let route1 = Route::new(EdgeId::new(), PortId::new(), PortId::new()).with_congestion(true);
        let route2 = Route::new(EdgeId::new(), PortId::new(), PortId::new());

        route_set.add_route(route1);
        route_set.add_route(route2);

        let congested = route_set.congested_routes();

        assert_eq!(congested.len(), 1);
    }

    #[test]
    fn test_route_set_simplify_all() {
        let mut route_set = RouteSet::new();

        let route = Route::from_segments(
            EdgeId::new(),
            PortId::new(),
            PortId::new(),
            vec![
                RouteSegment::move_to(0.0, 0.0),
                RouteSegment::line_to(5.0, 0.0),
                RouteSegment::line_to(10.0, 0.0),
            ],
        );

        route_set.add_route(route);
        route_set.simplify_all();

        assert_eq!(route_set.routes()[0].segments.len(), 2);
    }

    #[test]
    fn test_route_set_clear() {
        let mut route_set = RouteSet::new();
        route_set.add_route(Route::new(EdgeId::new(), PortId::new(), PortId::new()));

        route_set.clear();

        assert!(route_set.is_empty());
    }

    #[test]
    fn test_route_stats_empty() {
        let routes: Vec<Route> = vec![];
        let stats = RouteStats::from_routes(&routes);

        assert_eq!(stats.route_count, 0);
        assert_eq!(stats.total_length, 0.0);
        assert_eq!(stats.total_bends, 0);
        assert_eq!(stats.avg_length, 0.0);
        assert_eq!(stats.avg_bends, 0.0);
    }

    #[test]
    fn test_route_stats_max_bends() {
        let routes = vec![Route::from_segments(
            EdgeId::new(),
            PortId::new(),
            PortId::new(),
            vec![
                RouteSegment::move_to(0.0, 0.0),
                RouteSegment::line_to(10.0, 0.0),
                RouteSegment::line_to(10.0, 10.0),
                RouteSegment::line_to(20.0, 10.0),
                RouteSegment::line_to(20.0, 20.0),
            ],
        )];

        let stats = RouteStats::from_routes(&routes);

        assert_eq!(stats.max_bends, 3);
    }

    #[test]
    fn test_route_stats_congested_count() {
        let routes = vec![
            Route::new(EdgeId::new(), PortId::new(), PortId::new()).with_congestion(true),
            Route::new(EdgeId::new(), PortId::new(), PortId::new()).with_congestion(true),
            Route::new(EdgeId::new(), PortId::new(), PortId::new()),
        ];

        let stats = RouteStats::from_routes(&routes);

        assert_eq!(stats.congested_count, 2);
    }

    // ========== Tests for new edge info fields ==========

    #[test]
    fn test_route_new_default_edge_info() {
        let route = Route::new(EdgeId::new(), PortId::new(), PortId::new());

        assert!(route.label.is_none());
        assert_eq!(route.edge_kind, EdgeKind::default());
        assert_eq!(route.edge_color, Color::GRAY_500);
    }

    #[test]
    fn test_route_with_label() {
        let route =
            Route::new(EdgeId::new(), PortId::new(), PortId::new()).with_label("test edge label");

        assert_eq!(route.label, Some("test edge label".to_string()));
    }

    #[test]
    fn test_route_with_edge_kind() {
        let route = Route::new(EdgeId::new(), PortId::new(), PortId::new())
            .with_edge_kind(EdgeKind::Dashed);

        assert_eq!(route.edge_kind, EdgeKind::Dashed);
    }

    #[test]
    fn test_route_with_edge_color() {
        let route =
            Route::new(EdgeId::new(), PortId::new(), PortId::new()).with_edge_color(Color::RED);

        assert_eq!(route.edge_color, Color::RED);
    }

    #[test]
    fn test_route_builder_chain() {
        let route = Route::new(EdgeId::new(), PortId::new(), PortId::new())
            .with_label("full edge")
            .with_edge_kind(EdgeKind::Bold)
            .with_edge_color(Color::BLUE)
            .with_congestion(true);

        assert_eq!(route.label, Some("full edge".to_string()));
        assert_eq!(route.edge_kind, EdgeKind::Bold);
        assert_eq!(route.edge_color, Color::BLUE);
        assert!(route.crosses_congestion);
    }
}
