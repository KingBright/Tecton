use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// A 2D point with ordered float coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    pub x: OrderedFloat<f32>,
    pub y: OrderedFloat<f32>,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: OrderedFloat(x),
            y: OrderedFloat(y),
        }
    }

    pub fn x(&self) -> f32 {
        self.x.into_inner()
    }

    pub fn y(&self) -> f32 {
        self.y.into_inner()
    }

    pub fn distance_to(&self, other: &Point) -> f32 {
        let dx = self.x() - other.x();
        let dy = self.y() - other.y();
        (dx * dx + dy * dy).sqrt()
    }

    pub fn manhattan_distance_to(&self, other: &Point) -> f32 {
        (self.x() - other.x()).abs() + (self.y() - other.y()).abs()
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x() + rhs.x(), self.y() + rhs.y())
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x() - rhs.x(), self.y() - rhs.y())
    }
}

/// A 2D size with ordered float dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Size {
    pub width: OrderedFloat<f32>,
    pub height: OrderedFloat<f32>,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: OrderedFloat(width),
            height: OrderedFloat(height),
        }
    }

    pub fn width(&self) -> f32 {
        self.width.into_inner()
    }

    pub fn height(&self) -> f32 {
        self.height.into_inner()
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height() == 0.0 {
            return f32::INFINITY;
        }
        self.width() / self.height()
    }

    pub fn is_empty(&self) -> bool {
        self.width() <= 0.0 || self.height() <= 0.0
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// A rectangle defined by position and size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn from_origin_size(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    pub fn x(&self) -> f32 {
        self.origin.x()
    }

    pub fn y(&self) -> f32 {
        self.origin.y()
    }

    pub fn width(&self) -> f32 {
        self.size.width()
    }

    pub fn height(&self) -> f32 {
        self.size.height()
    }

    pub fn left(&self) -> f32 {
        self.x()
    }

    pub fn right(&self) -> f32 {
        self.x() + self.width()
    }

    pub fn top(&self) -> f32 {
        self.y()
    }

    pub fn bottom(&self) -> f32 {
        self.y() + self.height()
    }

    pub fn center(&self) -> Point {
        Point::new(
            self.x() + self.width() / 2.0,
            self.y() + self.height() / 2.0,
        )
    }

    pub fn top_left(&self) -> Point {
        self.origin
    }

    pub fn top_right(&self) -> Point {
        Point::new(self.right(), self.y())
    }

    pub fn bottom_left(&self) -> Point {
        Point::new(self.x(), self.bottom())
    }

    pub fn bottom_right(&self) -> Point {
        Point::new(self.right(), self.bottom())
    }

    pub fn contains(&self, point: &Point) -> bool {
        point.x() >= self.left()
            && point.x() <= self.right()
            && point.y() >= self.top()
            && point.y() <= self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.left() < other.right()
            && self.right() > other.left()
            && self.top() < other.bottom()
            && self.bottom() > other.top()
    }

    pub fn inset(&self, dx: f32, dy: f32) -> Self {
        Self::new(
            self.x() + dx,
            self.y() + dy,
            self.width() - 2.0 * dx,
            self.height() - 2.0 * dy,
        )
    }

    pub fn offset(&self, dx: f32, dy: f32) -> Self {
        Self::new(self.x() + dx, self.y() + dy, self.width(), self.height())
    }

    pub fn union(&self, other: &Rect) -> Self {
        let left = self.left().min(other.left());
        let top = self.top().min(other.top());
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Self::new(left, top, right - left, bottom - top)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

/// A port (connection point) on a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortSide {
    North,
    South,
    East,
    West,
}

impl PortSide {
    pub fn opposite(&self) -> Self {
        match self {
            PortSide::North => PortSide::South,
            PortSide::South => PortSide::North,
            PortSide::East => PortSide::West,
            PortSide::West => PortSide::East,
        }
    }
}

/// Position of a port on a node's boundary
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Port {
    pub side: PortSide,
    /// Normalized position along the side (0.0 to 1.0)
    pub position: OrderedFloat<f32>,
}

impl Port {
    pub fn new(side: PortSide, position: f32) -> Self {
        Self {
            side,
            position: OrderedFloat(position.clamp(0.0, 1.0)),
        }
    }

    pub fn position(&self) -> f32 {
        self.position.into_inner()
    }

    /// Calculate the absolute position of this port on a given rectangle
    pub fn absolute_position(&self, rect: &Rect) -> Point {
        match self.side {
            PortSide::North => Point::new(rect.left() + rect.width() * self.position(), rect.top()),
            PortSide::South => {
                Point::new(rect.left() + rect.width() * self.position(), rect.bottom())
            }
            PortSide::East => {
                Point::new(rect.right(), rect.top() + rect.height() * self.position())
            }
            PortSide::West => Point::new(rect.left(), rect.top() + rect.height() * self.position()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(rect.contains(&Point::new(50.0, 50.0)));
        assert!(rect.contains(&Point::new(0.0, 0.0)));
        assert!(rect.contains(&Point::new(100.0, 100.0)));
        assert!(!rect.contains(&Point::new(101.0, 50.0)));
    }

    #[test]
    fn test_rect_intersects() {
        let rect1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = Rect::new(50.0, 50.0, 100.0, 100.0);
        let rect3 = Rect::new(200.0, 200.0, 100.0, 100.0);

        assert!(rect1.intersects(&rect2));
        assert!(!rect1.intersects(&rect3));
    }

    #[test]
    fn test_port_absolute_position() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);

        let north_port = Port::new(PortSide::North, 0.5);
        assert_eq!(north_port.absolute_position(&rect), Point::new(50.0, 0.0));

        let east_port = Port::new(PortSide::East, 0.5);
        assert_eq!(east_port.absolute_position(&rect), Point::new(100.0, 50.0));
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_point_add_sub() {
        let p1 = Point::new(10.0, 20.0);
        let p2 = Point::new(5.0, 15.0);

        let sum = p1 + p2;
        assert_eq!(sum, Point::new(15.0, 35.0));

        let diff = p1 - p2;
        assert_eq!(diff, Point::new(5.0, 5.0));
    }

    #[test]
    fn test_point_distance_to() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);

        assert!((p1.distance_to(&p2) - 5.0).abs() < 0.001);
        assert!((p1.manhattan_distance_to(&p2) - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_point_distance_symmetry() {
        let p1 = Point::new(10.0, 20.0);
        let p2 = Point::new(30.0, 40.0);

        assert!((p1.distance_to(&p2) - p2.distance_to(&p1)).abs() < 0.001);
    }

    #[test]
    fn test_size_zero() {
        let size = Size::new(0.0, 0.0);
        assert!(size.is_empty());
        assert_eq!(size.area(), 0.0);
    }

    #[test]
    fn test_size_area() {
        let size = Size::new(10.0, 20.0);
        assert_eq!(size.area(), 200.0);
    }

    #[test]
    fn test_size_aspect_ratio() {
        let size = Size::new(100.0, 50.0);
        assert!((size.aspect_ratio() - 2.0).abs() < 0.001);

        // Zero height should return infinity
        let zero_height = Size::new(100.0, 0.0);
        assert!(zero_height.aspect_ratio().is_infinite());
    }

    #[test]
    fn test_size_default() {
        let size = Size::default();
        assert!(size.is_empty());
    }

    #[test]
    fn test_rect_zero_size() {
        let rect = Rect::new(10.0, 20.0, 0.0, 0.0);
        assert_eq!(rect.width(), 0.0);
        assert_eq!(rect.height(), 0.0);
        assert!(rect.size.is_empty());
    }

    #[test]
    fn test_rect_center() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        let center = rect.center();
        assert_eq!(center, Point::new(50.0, 25.0));
    }

    #[test]
    fn test_rect_corners() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);

        assert_eq!(rect.top_left(), Point::new(10.0, 20.0));
        assert_eq!(rect.top_right(), Point::new(110.0, 20.0));
        assert_eq!(rect.bottom_left(), Point::new(10.0, 70.0));
        assert_eq!(rect.bottom_right(), Point::new(110.0, 70.0));
    }

    #[test]
    fn test_rect_inset() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let inset = rect.inset(10.0, 5.0);

        assert_eq!(inset.x(), 10.0);
        assert_eq!(inset.y(), 5.0);
        assert_eq!(inset.width(), 80.0);
        assert_eq!(inset.height(), 90.0);
    }

    #[test]
    fn test_rect_offset() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let offset = rect.offset(50.0, 25.0);

        assert_eq!(offset.x(), 50.0);
        assert_eq!(offset.y(), 25.0);
        assert_eq!(offset.width(), 100.0);
        assert_eq!(offset.height(), 100.0);
    }

    #[test]
    fn test_rect_union() {
        let rect1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = Rect::new(50.0, 50.0, 100.0, 100.0);
        let union = rect1.union(&rect2);

        assert_eq!(union.x(), 0.0);
        assert_eq!(union.y(), 0.0);
        assert_eq!(union.width(), 150.0);
        assert_eq!(union.height(), 150.0);
    }

    #[test]
    fn test_rect_union_superset() {
        let rect1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = Rect::new(200.0, 200.0, 50.0, 50.0);
        let union = rect1.union(&rect2);

        // Union should contain both original rectangles
        assert!(union.contains(&rect1.top_left()));
        assert!(union.contains(&rect1.bottom_right()));
        assert!(union.contains(&rect2.top_left()));
        assert!(union.contains(&rect2.bottom_right()));
    }

    #[test]
    fn test_rect_default() {
        let rect = Rect::default();
        assert_eq!(rect.x(), 0.0);
        assert_eq!(rect.y(), 0.0);
        assert_eq!(rect.width(), 0.0);
        assert_eq!(rect.height(), 0.0);
    }

    #[test]
    fn test_rect_from_origin_size() {
        let origin = Point::new(10.0, 20.0);
        let size = Size::new(100.0, 50.0);
        let rect = Rect::from_origin_size(origin, size);

        assert_eq!(rect.x(), 10.0);
        assert_eq!(rect.y(), 20.0);
        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 50.0);
    }

    #[test]
    fn test_rect_edge_tangency() {
        // Two rectangles that touch at an edge
        // Note: intersects() uses strict inequality, so edge-touching rects do NOT intersect
        let rect1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = Rect::new(100.0, 0.0, 100.0, 100.0);

        // Edge-touching rectangles do NOT intersect by this implementation
        assert!(!rect1.intersects(&rect2));

        // But rects that overlap slightly should intersect
        let rect3 = Rect::new(99.0, 0.0, 100.0, 100.0);
        assert!(rect1.intersects(&rect3));
    }

    #[test]
    fn test_port_side_opposite() {
        assert_eq!(PortSide::North.opposite(), PortSide::South);
        assert_eq!(PortSide::South.opposite(), PortSide::North);
        assert_eq!(PortSide::East.opposite(), PortSide::West);
        assert_eq!(PortSide::West.opposite(), PortSide::East);
    }

    #[test]
    fn test_port_clamp_position() {
        let port = Port::new(PortSide::North, 1.5); // Position > 1.0
        assert!((port.position() - 1.0).abs() < 0.001); // Should be clamped to 1.0

        let port2 = Port::new(PortSide::North, -0.5); // Position < 0.0
        assert!((port2.position() - 0.0).abs() < 0.001); // Should be clamped to 0.0
    }

    #[test]
    fn test_port_all_sides() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);

        let north = Port::new(PortSide::North, 0.5);
        let south = Port::new(PortSide::South, 0.5);
        let east = Port::new(PortSide::East, 0.5);
        let west = Port::new(PortSide::West, 0.5);

        assert_eq!(north.absolute_position(&rect), Point::new(50.0, 0.0));
        assert_eq!(south.absolute_position(&rect), Point::new(50.0, 100.0));
        assert_eq!(east.absolute_position(&rect), Point::new(100.0, 50.0));
        assert_eq!(west.absolute_position(&rect), Point::new(0.0, 50.0));
    }

    #[test]
    fn test_rect_contains_center() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let center = rect.center();
        assert!(rect.contains(&center));
    }

    #[test]
    fn test_rect_bounds() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);

        assert_eq!(rect.left(), 10.0);
        assert_eq!(rect.right(), 110.0);
        assert_eq!(rect.top(), 20.0);
        assert_eq!(rect.bottom(), 70.0);
    }
}
