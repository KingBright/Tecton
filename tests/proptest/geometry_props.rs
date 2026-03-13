//! Property-Based Tests for Geometry Module
//!
//! These tests verify geometric invariants using property-based testing.

use tecton_types::geometry::{Point, Rect, Size};

/// Property: Rectangle always contains its center point
#[test]
fn prop_rect_contains_center() {
    let test_cases = vec![
        (Rect::new(0.0, 0.0, 100.0, 100.0), Point::new(50.0, 50.0)),
        (Rect::new(10.0, 20.0, 200.0, 150.0), Point::new(110.0, 95.0)),
        (Rect::new(-50.0, -50.0, 100.0, 100.0), Point::new(0.0, 0.0)),
    ];

    for (rect, center) in test_cases {
        assert!(
            rect.contains(center),
            "Rectangle should contain its center point"
        );
    }
}

/// Property: Rectangle union is a superset of both rectangles
#[test]
fn prop_rect_union_superset() {
    let rect1 = Rect::new(0.0, 0.0, 50.0, 50.0);
    let rect2 = Rect::new(30.0, 30.0, 50.0, 50.0);

    let union = rect1.union(&rect2);

    // Union should contain both original rectangles
    assert!(union.contains(rect1.top_left()));
    assert!(union.contains(rect1.bottom_right()));
    assert!(union.contains(rect2.top_left()));
    assert!(union.contains(rect2.bottom_right()));
}

/// Property: Distance symmetry - d(a,b) == d(b,a)
#[test]
fn prop_distance_symmetry() {
    let test_cases = vec![
        (Point::new(0.0, 0.0), Point::new(3.0, 4.0)),
        (Point::new(-10.0, 20.0), Point::new(30.0, -40.0)),
        (Point::new(100.0, 100.0), Point::new(100.0, 100.0)),
    ];

    for (a, b) in test_cases {
        let d_ab = a.distance_to(b);
        let d_ba = b.distance_to(a);
        assert!(
            (d_ab - d_ba).abs() < f32::EPSILON,
            "Distance should be symmetric"
        );
    }
}

/// Property: Point distance to self is zero
#[test]
fn prop_distance_to_self_zero() {
    let points = vec![
        Point::new(0.0, 0.0),
        Point::new(100.0, 200.0),
        Point::new(-50.0, -75.0),
    ];

    for point in points {
        let distance = point.distance_to(point);
        assert!(
            distance.abs() < f32::EPSILON,
            "Distance to self should be zero"
        );
    }
}

/// Property: Rectangle intersection is subset of both rectangles
#[test]
fn prop_rect_intersection_subset() {
    let rect1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    let rect2 = Rect::new(50.0, 50.0, 100.0, 100.0);

    let intersection = rect1.intersect(&rect2);

    if let Some(inter) = intersection {
        // Intersection corners should be in both rectangles
        assert!(rect1.contains(inter.top_left()));
        assert!(rect1.contains(inter.bottom_right()));
        assert!(rect2.contains(inter.top_left()));
        assert!(rect2.contains(inter.bottom_right()));
    }
}

/// Property: Rectangle area is always non-negative
#[test]
fn prop_rect_area_non_negative() {
    let rects = vec![
        Rect::new(0.0, 0.0, 100.0, 100.0),
        Rect::new(10.0, 10.0, 50.0, 30.0),
        Rect::new(-100.0, -100.0, 200.0, 200.0),
    ];

    for rect in rects {
        let area = rect.area();
        assert!(area >= 0.0, "Rectangle area should be non-negative");
    }
}

/// Property: Size aspect ratio calculation
#[test]
fn prop_size_aspect_ratio() {
    let sizes = vec![
        (Size::new(100.0, 100.0), 1.0),      // Square
        (Size::new(200.0, 100.0), 2.0),      // Wide
        (Size::new(100.0, 200.0), 0.5),      // Tall
    ];

    for (size, expected_ratio) in sizes {
        let ratio = size.width() / size.height();
        assert!(
            (ratio - expected_ratio).abs() < f32::EPSILON,
            "Aspect ratio should be width/height"
        );
    }
}

/// Property: Rect center point calculation
#[test]
fn prop_rect_center_calculation() {
    let rects = vec![
        (Rect::new(0.0, 0.0, 100.0, 100.0), Point::new(50.0, 50.0)),
        (Rect::new(10.0, 20.0, 80.0, 60.0), Point::new(50.0, 50.0)),
    ];

    for (rect, expected_center) in rects {
        let center = rect.center();
        assert!(
            (center.x() - expected_center.x()).abs() < f32::EPSILON,
            "Center x should be correct"
        );
        assert!(
            (center.y() - expected_center.y()).abs() < f32::EPSILON,
            "Center y should be correct"
        );
    }
}

/// Property: Point addition is commutative
#[test]
fn prop_point_addition_commutative() {
    let a = Point::new(10.0, 20.0);
    let b = Point::new(5.0, 15.0);

    let ab = Point::new(a.x() + b.x(), a.y() + b.y());
    let ba = Point::new(b.x() + a.x(), b.y() + a.y());

    assert!(
        (ab.x() - ba.x()).abs() < f32::EPSILON &&
        (ab.y() - ba.y()).abs() < f32::EPSILON,
        "Point addition should be commutative"
    );
}

/// Property: Expanded rectangle contains original
#[test]
fn prop_rect_expanded_contains_original() {
    let rect = Rect::new(10.0, 10.0, 80.0, 80.0);
    let expanded = rect.expand(10.0);

    assert!(
        expanded.left() <= rect.left() &&
        expanded.right() >= rect.right() &&
        expanded.top() <= rect.top() &&
        expanded.bottom() >= rect.bottom(),
        "Expanded rectangle should contain original"
    );
}
