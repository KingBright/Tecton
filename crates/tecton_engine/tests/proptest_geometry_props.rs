//! Property-based tests for geometry invariants

use tecton_types::geometry::{Point, Rect, Size};

#[test]
fn prop_rect_contains_center() {
    // Property: A rectangle always contains its center point
    for i in 0..100 {
        let x = (i % 10) as f32 * 10.0;
        let y = (i / 10) as f32 * 10.0;
        let width = 50.0 + (i as f32 % 50.0);
        let height = 30.0 + (i as f32 % 30.0);

        let rect = Rect::new(x, y, width, height);
        let center = rect.center();

        assert!(
            rect.contains(&center),
            "Rect {:?} should contain its center {:?}",
            rect,
            center
        );
    }
}

#[test]
fn prop_rect_union_superset() {
    // Property: The union of two rectangles contains both original rectangles
    for i in 0..50 {
        let rect1 = Rect::new((i % 5) as f32 * 20.0, (i / 5) as f32 * 20.0, 50.0, 30.0);
        let rect2 = Rect::new(
            ((i + 2) % 5) as f32 * 20.0,
            ((i + 3) / 5) as f32 * 20.0,
            40.0,
            25.0,
        );

        let union = rect1.union(&rect2);

        // Union should contain all corners of both rectangles
        assert!(
            union.contains(&rect1.top_left()),
            "Union should contain rect1 top_left"
        );
        assert!(
            union.contains(&rect1.bottom_right()),
            "Union should contain rect1 bottom_right"
        );
        assert!(
            union.contains(&rect2.top_left()),
            "Union should contain rect2 top_left"
        );
        assert!(
            union.contains(&rect2.bottom_right()),
            "Union should contain rect2 bottom_right"
        );
    }
}

#[test]
fn prop_distance_symmetry() {
    // Property: Distance from A to B equals distance from B to A
    for i in 0..100 {
        let p1 = Point::new((i % 10) as f32 * 10.0, (i / 10) as f32 * 10.0);
        let p2 = Point::new(((i + 3) % 10) as f32 * 10.0, ((i + 7) / 10) as f32 * 10.0);

        let d1 = p1.distance_to(&p2);
        let d2 = p2.distance_to(&p1);

        assert!(
            (d1 - d2).abs() < 0.001,
            "Distance should be symmetric: {} vs {}",
            d1,
            d2
        );
    }
}

#[test]
fn prop_rect_inset_smaller() {
    // Property: An inset rectangle is always smaller than the original
    for i in 0..50 {
        let dx = (i % 5) as f32;
        let dy = (i / 5) as f32;
        let rect = Rect::new(0.0, 0.0, 100.0, 80.0);

        let inset = rect.inset(dx, dy);

        if dx > 0.0 || dy > 0.0 {
            assert!(
                inset.size.width < rect.size.width || inset.size.height < rect.size.height,
                "Inset should be smaller"
            );
        }
    }
}

#[test]
fn prop_size_area_consistency() {
    // Property: width * height equals area
    for i in 1..100 {
        let width = (i % 10) as f32 * 10.0;
        let height = (i / 10) as f32 * 10.0;
        let size = Size::new(width, height);

        let expected_area = width * height;
        let actual_area = size.area();

        assert!(
            (expected_area - actual_area).abs() < 0.001,
            "Area should equal width * height"
        );
    }
}

#[test]
fn prop_point_add_sub_inverse() {
    // Property: Adding and then subtracting the same point returns to original
    for i in 0..50 {
        let p1 = Point::new((i % 10) as f32 * 10.0, (i / 10) as f32 * 10.0);
        let p2 = Point::new(((i + 3) % 10) as f32 * 5.0, ((i + 7) % 10) as f32 * 5.0);

        let result = p1 + p2 - p2;

        assert!(
            (result.x() - p1.x()).abs() < 0.001 && (result.y() - p1.y()).abs() < 0.001,
            "Add/Sub should be inverse operations"
        );
    }
}
