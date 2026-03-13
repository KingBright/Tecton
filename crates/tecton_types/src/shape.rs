use serde::{Deserialize, Serialize};

use crate::geometry::Point;
use crate::geometry::PortSide;

/// Direction for edge connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Top,
    Bottom,
    Left,
    Right,
}

impl Direction {
    pub fn from_port_side(side: PortSide) -> Self {
        match side {
            PortSide::North => Direction::Top,
            PortSide::South => Direction::Bottom,
            PortSide::East => Direction::Right,
            PortSide::West => Direction::Left,
        }
    }

    pub fn to_port_side(&self) -> PortSide {
        match self {
            Direction::Top => PortSide::North,
            Direction::Bottom => PortSide::South,
            Direction::Left => PortSide::West,
            Direction::Right => PortSide::East,
        }
    }
}

/// Shape types supported by Tecton
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShapeType {
    /// Rectangle (default)
    Rectangle,
    /// Circle or ellipse
    Circle,
    /// Diamond (rhombus) for decision nodes
    Diamond,
    /// Cylinder for databases
    Cylinder,
    /// Parallelogram for input/output
    Parallelogram,
    /// Hexagon
    Hexagon,
    /// Cloud shape
    Cloud,
}

impl Default for ShapeType {
    fn default() -> Self {
        ShapeType::Rectangle
    }
}

/// Trait for shape geometry calculations
pub trait ShapeGeometry {
    /// Get the layout engine's bounding box dimensions (width, height)
    fn measure(&self) -> (f32, f32);

    /// Calculate the connection point for an edge
    /// - center: The node's center point
    /// - direction: The direction from which the edge approaches
    fn get_port_position(&self, center: Point, direction: Direction) -> Point;

    /// Check if a point is inside the shape (for obstacle detection)
    fn contains_point(&self, center: Point, point: Point) -> bool;

    /// Get the shape type
    fn shape_type(&self) -> ShapeType;
}

/// Rectangle shape
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangleShape {
    pub width: f32,
    pub height: f32,
}

impl RectangleShape {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl ShapeGeometry for RectangleShape {
    fn measure(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    fn get_port_position(&self, center: Point, direction: Direction) -> Point {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        match direction {
            Direction::Top => Point::new(center.x(), center.y() - half_h),
            Direction::Bottom => Point::new(center.x(), center.y() + half_h),
            Direction::Left => Point::new(center.x() - half_w, center.y()),
            Direction::Right => Point::new(center.x() + half_w, center.y()),
        }
    }

    fn contains_point(&self, center: Point, point: Point) -> bool {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        point.x() >= center.x() - half_w
            && point.x() <= center.x() + half_w
            && point.y() >= center.y() - half_h
            && point.y() <= center.y() + half_h
    }

    fn shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }
}

/// Circle shape
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleShape {
    pub radius: f32,
}

impl CircleShape {
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }

    pub fn diameter(&self) -> f32 {
        self.radius * 2.0
    }
}

impl ShapeGeometry for CircleShape {
    fn measure(&self) -> (f32, f32) {
        let d = self.diameter();
        (d, d)
    }

    fn get_port_position(&self, center: Point, direction: Direction) -> Point {
        // For orthogonal layouts, connect at the four cardinal points
        match direction {
            Direction::Top => Point::new(center.x(), center.y() - self.radius),
            Direction::Bottom => Point::new(center.x(), center.y() + self.radius),
            Direction::Left => Point::new(center.x() - self.radius, center.y()),
            Direction::Right => Point::new(center.x() + self.radius, center.y()),
        }
    }

    fn contains_point(&self, center: Point, point: Point) -> bool {
        let dx = point.x() - center.x();
        let dy = point.y() - center.y();
        (dx * dx + dy * dy) <= self.radius * self.radius
    }

    fn shape_type(&self) -> ShapeType {
        ShapeType::Circle
    }
}

/// Diamond shape (rhombus) for decision nodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiamondShape {
    pub size: f32, // Width and height are the same for a square diamond
}

impl DiamondShape {
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

impl ShapeGeometry for DiamondShape {
    fn measure(&self) -> (f32, f32) {
        (self.size, self.size)
    }

    fn get_port_position(&self, center: Point, direction: Direction) -> Point {
        let half_size = self.size / 2.0;

        // Diamond vertices are at the midpoints of the bounding box edges
        match direction {
            Direction::Top => Point::new(center.x(), center.y() - half_size),
            Direction::Bottom => Point::new(center.x(), center.y() + half_size),
            Direction::Left => Point::new(center.x() - half_size, center.y()),
            Direction::Right => Point::new(center.x() + half_size, center.y()),
        }
    }

    fn contains_point(&self, center: Point, point: Point) -> bool {
        // Transform to local coordinates
        let dx = (point.x() - center.x()).abs();
        let dy = (point.y() - center.y()).abs();
        let half_size = self.size / 2.0;

        // Point is inside diamond if: dx/half_size + dy/half_size <= 1
        // Or equivalently: dx + dy <= half_size
        dx + dy <= half_size
    }

    fn shape_type(&self) -> ShapeType {
        ShapeType::Diamond
    }
}

/// Parallelogram shape
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParallelogramShape {
    pub width: f32,
    pub height: f32,
    pub skew: f32, // Horizontal offset of top edge (0 = rectangle)
}

impl ParallelogramShape {
    pub fn new(width: f32, height: f32, skew: f32) -> Self {
        Self {
            width,
            height,
            skew,
        }
    }
}

impl ShapeGeometry for ParallelogramShape {
    fn measure(&self) -> (f32, f32) {
        (self.width + self.skew.abs(), self.height)
    }

    fn get_port_position(&self, center: Point, direction: Direction) -> Point {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        match direction {
            Direction::Top => Point::new(center.x() + self.skew / 2.0, center.y() - half_h),
            Direction::Bottom => Point::new(center.x() - self.skew / 2.0, center.y() + half_h),
            Direction::Left => Point::new(center.x() - half_w - self.skew / 2.0, center.y()),
            Direction::Right => Point::new(center.x() + half_w + self.skew / 2.0, center.y()),
        }
    }

    fn contains_point(&self, center: Point, point: Point) -> bool {
        // Simplified containment check
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;
        let dx = point.x() - center.x();
        let dy = point.y() - center.y();

        dy.abs() <= half_h && dx.abs() <= half_w + (half_h - dy.abs()) * (self.skew.abs() / half_h)
    }

    fn shape_type(&self) -> ShapeType {
        ShapeType::Parallelogram
    }
}

/// Cylinder shape (for databases)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CylinderShape {
    pub width: f32,
    pub height: f32,
}

impl CylinderShape {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl ShapeGeometry for CylinderShape {
    fn measure(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    fn get_port_position(&self, center: Point, direction: Direction) -> Point {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        match direction {
            Direction::Top => Point::new(center.x(), center.y() - half_h),
            Direction::Bottom => Point::new(center.x(), center.y() + half_h),
            Direction::Left => Point::new(center.x() - half_w, center.y()),
            Direction::Right => Point::new(center.x() + half_w, center.y()),
        }
    }

    fn contains_point(&self, center: Point, point: Point) -> bool {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        point.x() >= center.x() - half_w
            && point.x() <= center.x() + half_w
            && point.y() >= center.y() - half_h
            && point.y() <= center.y() + half_h
    }

    fn shape_type(&self) -> ShapeType {
        ShapeType::Cylinder
    }
}

/// Shape factory for creating shapes
pub struct ShapeFactory;

impl ShapeFactory {
    pub fn create(shape_type: ShapeType, width: f32, height: f32) -> Box<dyn ShapeGeometry> {
        match shape_type {
            ShapeType::Rectangle => Box::new(RectangleShape::new(width, height)),
            ShapeType::Circle => {
                let radius = width.min(height) / 2.0;
                Box::new(CircleShape::new(radius))
            }
            ShapeType::Diamond => {
                let size = width.min(height);
                Box::new(DiamondShape::new(size))
            }
            ShapeType::Cylinder => Box::new(CylinderShape::new(width, height)),
            ShapeType::Parallelogram => {
                let skew = width * 0.2; // Default 20% skew
                Box::new(ParallelogramShape::new(width * 0.8, height, skew))
            }
            ShapeType::Hexagon => Box::new(RectangleShape::new(width, height)), // Fallback
            ShapeType::Cloud => Box::new(RectangleShape::new(width, height)),   // Fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rectangle_port_positions() {
        let rect = RectangleShape::new(100.0, 50.0);
        let center = Point::new(50.0, 25.0);

        assert_eq!(
            rect.get_port_position(center, Direction::Top),
            Point::new(50.0, 0.0)
        );
        assert_eq!(
            rect.get_port_position(center, Direction::Bottom),
            Point::new(50.0, 50.0)
        );
        assert_eq!(
            rect.get_port_position(center, Direction::Left),
            Point::new(0.0, 25.0)
        );
        assert_eq!(
            rect.get_port_position(center, Direction::Right),
            Point::new(100.0, 25.0)
        );
    }

    #[test]
    fn test_circle_port_positions() {
        let circle = CircleShape::new(50.0);
        let center = Point::new(100.0, 100.0);

        assert_eq!(
            circle.get_port_position(center, Direction::Top),
            Point::new(100.0, 50.0)
        );
        assert_eq!(
            circle.get_port_position(center, Direction::Right),
            Point::new(150.0, 100.0)
        );
    }

    #[test]
    fn test_diamond_contains_point() {
        let diamond = DiamondShape::new(100.0);
        let center = Point::new(0.0, 0.0);

        assert!(diamond.contains_point(center, Point::new(0.0, 0.0))); // Center
        assert!(diamond.contains_point(center, Point::new(20.0, 20.0))); // Inside
        assert!(!diamond.contains_point(center, Point::new(40.0, 40.0))); // Outside (corner)
    }

    #[test]
    fn test_shape_factory() {
        let rect = ShapeFactory::create(ShapeType::Rectangle, 100.0, 50.0);
        assert_eq!(rect.measure(), (100.0, 50.0));

        let circle = ShapeFactory::create(ShapeType::Circle, 100.0, 100.0);
        assert_eq!(circle.measure(), (100.0, 100.0));
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_rectangle_contains_point() {
        let rect = RectangleShape::new(100.0, 50.0);
        let center = Point::new(50.0, 25.0);

        // Points inside
        assert!(rect.contains_point(center, Point::new(50.0, 25.0))); // Center
        assert!(rect.contains_point(center, Point::new(0.0, 0.0))); // Top-left corner
        assert!(rect.contains_point(center, Point::new(100.0, 50.0))); // Bottom-right corner

        // Points outside
        assert!(!rect.contains_point(center, Point::new(-1.0, 25.0))); // Left of bounds
        assert!(!rect.contains_point(center, Point::new(101.0, 25.0))); // Right of bounds
    }

    #[test]
    fn test_rectangle_measure() {
        let rect = RectangleShape::new(100.0, 50.0);
        assert_eq!(rect.measure(), (100.0, 50.0));
        assert_eq!(rect.shape_type(), ShapeType::Rectangle);
    }

    #[test]
    fn test_circle_diameter() {
        let circle = CircleShape::new(50.0);
        assert_eq!(circle.diameter(), 100.0);
        assert_eq!(circle.radius, 50.0);
    }

    #[test]
    fn test_circle_contains_point() {
        let circle = CircleShape::new(50.0);
        let center = Point::new(100.0, 100.0);

        // Points inside
        assert!(circle.contains_point(center, Point::new(100.0, 100.0))); // Center
        assert!(circle.contains_point(center, Point::new(140.0, 100.0))); // Edge
        assert!(circle.contains_point(center, Point::new(130.0, 130.0))); // Inside

        // Points outside
        assert!(!circle.contains_point(center, Point::new(151.0, 100.0))); // Just outside
    }

    #[test]
    fn test_diamond_port_positions() {
        let diamond = DiamondShape::new(100.0);
        let center = Point::new(50.0, 50.0);

        assert_eq!(
            diamond.get_port_position(center, Direction::Top),
            Point::new(50.0, 0.0)
        );
        assert_eq!(
            diamond.get_port_position(center, Direction::Bottom),
            Point::new(50.0, 100.0)
        );
        assert_eq!(
            diamond.get_port_position(center, Direction::Left),
            Point::new(0.0, 50.0)
        );
        assert_eq!(
            diamond.get_port_position(center, Direction::Right),
            Point::new(100.0, 50.0)
        );
    }

    #[test]
    fn test_diamond_measure() {
        let diamond = DiamondShape::new(100.0);
        assert_eq!(diamond.measure(), (100.0, 100.0));
        assert_eq!(diamond.shape_type(), ShapeType::Diamond);
    }

    #[test]
    fn test_cylinder_port_positions() {
        let cylinder = CylinderShape::new(100.0, 50.0);
        let center = Point::new(50.0, 25.0);

        assert_eq!(
            cylinder.get_port_position(center, Direction::Top),
            Point::new(50.0, 0.0)
        );
        assert_eq!(
            cylinder.get_port_position(center, Direction::Bottom),
            Point::new(50.0, 50.0)
        );
        assert_eq!(
            cylinder.get_port_position(center, Direction::Left),
            Point::new(0.0, 25.0)
        );
        assert_eq!(
            cylinder.get_port_position(center, Direction::Right),
            Point::new(100.0, 25.0)
        );
    }

    #[test]
    fn test_cylinder_contains_point() {
        let cylinder = CylinderShape::new(100.0, 50.0);
        let center = Point::new(50.0, 25.0);

        assert!(cylinder.contains_point(center, Point::new(50.0, 25.0))); // Center
        assert!(cylinder.contains_point(center, Point::new(0.0, 0.0))); // Corner
    }

    #[test]
    fn test_cylinder_measure() {
        let cylinder = CylinderShape::new(100.0, 50.0);
        assert_eq!(cylinder.measure(), (100.0, 50.0));
        assert_eq!(cylinder.shape_type(), ShapeType::Cylinder);
    }

    #[test]
    fn test_parallelogram_port_positions() {
        let para = ParallelogramShape::new(100.0, 50.0, 20.0);
        let center = Point::new(60.0, 25.0); // Center accounts for skew

        // Ports should be offset by skew
        let top_port = para.get_port_position(center, Direction::Top);
        assert!((top_port.y() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_parallelogram_measure() {
        let para = ParallelogramShape::new(100.0, 50.0, 20.0);
        let (w, h) = para.measure();

        // Width should include skew offset
        assert_eq!(w, 120.0); // width + abs(skew)
        assert_eq!(h, 50.0);
        assert_eq!(para.shape_type(), ShapeType::Parallelogram);
    }

    #[test]
    fn test_parallelogram_contains_point() {
        let para = ParallelogramShape::new(100.0, 50.0, 20.0);
        let center = Point::new(60.0, 25.0);

        // Center should be inside
        assert!(para.contains_point(center, center));
    }

    #[test]
    fn test_shape_factory_all_types() {
        // Test all shape types from factory
        let rect = ShapeFactory::create(ShapeType::Rectangle, 100.0, 50.0);
        assert_eq!(rect.shape_type(), ShapeType::Rectangle);

        let circle = ShapeFactory::create(ShapeType::Circle, 100.0, 100.0);
        assert_eq!(circle.shape_type(), ShapeType::Circle);

        let diamond = ShapeFactory::create(ShapeType::Diamond, 100.0, 100.0);
        assert_eq!(diamond.shape_type(), ShapeType::Diamond);

        let cylinder = ShapeFactory::create(ShapeType::Cylinder, 100.0, 50.0);
        assert_eq!(cylinder.shape_type(), ShapeType::Cylinder);

        let parallelogram = ShapeFactory::create(ShapeType::Parallelogram, 100.0, 50.0);
        assert_eq!(parallelogram.shape_type(), ShapeType::Parallelogram);

        // Fallback shapes
        let hexagon = ShapeFactory::create(ShapeType::Hexagon, 100.0, 50.0);
        assert_eq!(hexagon.measure(), (100.0, 50.0));

        let cloud = ShapeFactory::create(ShapeType::Cloud, 100.0, 50.0);
        assert_eq!(cloud.measure(), (100.0, 50.0));
    }

    #[test]
    fn test_shape_type_default() {
        assert_eq!(ShapeType::default(), ShapeType::Rectangle);
    }

    #[test]
    fn test_direction_from_port_side() {
        assert_eq!(Direction::from_port_side(PortSide::North), Direction::Top);
        assert_eq!(
            Direction::from_port_side(PortSide::South),
            Direction::Bottom
        );
        assert_eq!(Direction::from_port_side(PortSide::East), Direction::Right);
        assert_eq!(Direction::from_port_side(PortSide::West), Direction::Left);
    }

    #[test]
    fn test_direction_to_port_side() {
        assert_eq!(Direction::Top.to_port_side(), PortSide::North);
        assert_eq!(Direction::Bottom.to_port_side(), PortSide::South);
        assert_eq!(Direction::Right.to_port_side(), PortSide::East);
        assert_eq!(Direction::Left.to_port_side(), PortSide::West);
    }

    #[test]
    fn test_circle_smallest_dimension() {
        // Circle uses min(width, height) for diameter
        let circle = ShapeFactory::create(ShapeType::Circle, 100.0, 50.0);
        assert_eq!(circle.measure(), (50.0, 50.0)); // min(100, 50) = 50
    }

    #[test]
    fn test_diamond_smallest_dimension() {
        // Diamond uses min(width, height) for size
        let diamond = ShapeFactory::create(ShapeType::Diamond, 100.0, 50.0);
        assert_eq!(diamond.measure(), (50.0, 50.0)); // min(100, 50) = 50
    }

    #[test]
    fn test_diamond_edge_contains() {
        let diamond = DiamondShape::new(100.0);
        let center = Point::new(0.0, 0.0);

        // Points on the edges should be inside
        assert!(diamond.contains_point(center, Point::new(25.0, 0.0))); // Right edge
        assert!(diamond.contains_point(center, Point::new(0.0, 25.0))); // Bottom edge
    }
}
