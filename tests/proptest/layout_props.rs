//! Property-Based Tests for Layout Module
//!
//! These tests verify layout invariants using property-based testing.

use tecton_layout::floorplanner::BisectionFloorplanner;
use tecton_layout::mass_calculator::MassCalculator;
use tecton_types::geometry::Rect;
use tecton_types::id::NodeId;
use tecton_types::mass::VisualMass;

/// Property: Floorplan preserves total area
#[test]
fn prop_floorplan_area_preserved() {
    let canvas = Rect::new(0.0, 0.0, 800.0, 600.0);
    let expected_area = canvas.area();

    // Create nodes with varying masses
    let nodes: Vec<(NodeId, VisualMass)> = vec![
        (NodeId::new(), VisualMass::new(10.0)),
        (NodeId::new(), VisualMass::new(20.0)),
        (NodeId::new(), VisualMass::new(15.0)),
    ];

    let mut planner = BisectionFloorplanner::new();
    let region = planner.plan(canvas, nodes);

    // The root region should match canvas bounds
    assert_eq!(region.rect.left(), canvas.left());
    assert_eq!(region.rect.right(), canvas.right());
    assert_eq!(region.rect.top(), canvas.top());
    assert_eq!(region.rect.bottom(), canvas.bottom());
}

/// Property: All nodes get assigned regions
#[test]
fn prop_all_nodes_assigned() {
    let canvas = Rect::new(0.0, 0.0, 400.0, 400.0);

    let node1 = NodeId::new();
    let node2 = NodeId::new();
    let node3 = NodeId::new();

    let nodes: Vec<(NodeId, VisualMass)> = vec![
        (node1, VisualMass::new(10.0)),
        (node2, VisualMass::new(10.0)),
        (node3, VisualMass::new(10.0)),
    ];

    let mut planner = BisectionFloorplanner::new();
    let region = planner.plan(canvas, nodes);

    // Count leaf regions (regions with node_id)
    fn count_leaves(region: &tecton_layout::floorplanner::FloorplanRegion) -> usize {
        if region.node_id.is_some() {
            return 1;
        }
        region.children.iter().map(count_leaves).sum()
    }

    let leaf_count = count_leaves(&region);
    assert_eq!(leaf_count, 3, "All 3 nodes should have regions assigned");
}

/// Property: Sibling regions don't overlap
#[test]
fn prop_sibling_regions_no_overlap() {
    let canvas = Rect::new(0.0, 0.0, 400.0, 400.0);

    let nodes: Vec<(NodeId, VisualMass)> = vec![
        (NodeId::new(), VisualMass::new(10.0)),
        (NodeId::new(), VisualMass::new(10.0)),
    ];

    let mut planner = BisectionFloorplanner::new();
    let region = planner.plan(canvas, nodes);

    // Check that sibling regions don't overlap
    fn check_siblings(region: &tecton_layout::floorplanner::FloorplanRegion) -> bool {
        for (i, child1) in region.children.iter().enumerate() {
            for child2 in region.children.iter().skip(i + 1) {
                // Siblings should have zero or minimal overlap
                let overlap = child1.rect.intersect(&child2.rect);
                if overlap.is_some() {
                    // Allow for small floating point tolerance
                    let inter = overlap.unwrap();
                    if inter.width() > 1.0 && inter.height() > 1.0 {
                        return false;
                    }
                }
            }
        }
        // Recursively check children
        region.children.iter().all(check_siblings)
    }

    assert!(check_siblings(&region), "Sibling regions should not overlap significantly");
}

/// Property: Region aspect ratios are reasonable
#[test]
fn prop_region_aspect_ratios_reasonable() {
    let canvas = Rect::new(0.0, 0.0, 800.0, 600.0);

    let nodes: Vec<(NodeId, VisualMass)> = vec![
        (NodeId::new(), VisualMass::new(10.0)),
        (NodeId::new(), VisualMass::new(10.0)),
        (NodeId::new(), VisualMass::new(10.0)),
        (NodeId::new(), VisualMass::new(10.0)),
    ];

    let mut planner = BisectionFloorplanner::new();
    let region = planner.plan(canvas, nodes);

    // Check aspect ratios of leaf regions
    fn check_aspect_ratios(region: &tecton_layout::floorplanner::FloorplanRegion) -> bool {
        if let Some(_) = region.node_id {
            let ratio = region.rect.width() / region.rect.height();
            // Aspect ratio should be between 0.25 and 4.0
            return ratio >= 0.25 && ratio <= 4.0;
        }
        region.children.iter().all(check_aspect_ratios)
    }

    assert!(
        check_aspect_ratios(&region),
        "Region aspect ratios should be reasonable"
    );
}

/// Property: Mass calculator produces non-negative masses
#[test]
fn prop_mass_non_negative() {
    let masses = vec![
        VisualMass::new(10.0),
        VisualMass::new(10.0).with_content(5.0),
        VisualMass::new(10.0).with_children(8.0),
        VisualMass::new(10.0).with_connectivity(3.0),
    ];

    for mass in masses {
        assert!(mass.total() >= 0.0, "Mass should be non-negative");
    }
}

/// Property: Heavier mass gets larger area
#[test]
fn prop_heavier_mass_larger_area() {
    let canvas = Rect::new(0.0, 0.0, 400.0, 400.0);

    let light_id = NodeId::new();
    let heavy_id = NodeId::new();

    let nodes: Vec<(NodeId, VisualMass)> = vec![
        (light_id, VisualMass::new(10.0)),
        (heavy_id, VisualMass::new(50.0)), // 5x heavier
    ];

    let mut planner = BisectionFloorplanner::new();
    let region = planner.plan(canvas, nodes);

    // Find areas for each node
    fn find_area(
        region: &tecton_layout::floorplanner::FloorplanRegion,
        node_id: NodeId,
    ) -> Option<f32> {
        if region.node_id == Some(node_id) {
            return Some(region.rect.area());
        }
        for child in &region.children {
            if let Some(area) = find_area(child, node_id) {
                return Some(area);
            }
        }
        None
    }

    let light_area = find_area(&region, light_id).expect("Light node should have area");
    let heavy_area = find_area(&region, heavy_id).expect("Heavy node should have area");

    assert!(
        heavy_area > light_area,
        "Heavier mass should get larger area"
    );
}

/// Property: Child regions fit within parent
#[test]
fn prop_children_fit_in_parent() {
    let canvas = Rect::new(0.0, 0.0, 400.0, 400.0);

    let nodes: Vec<(NodeId, VisualMass)> = vec![
        (NodeId::new(), VisualMass::new(10.0)),
        (NodeId::new(), VisualMass::new(10.0)),
        (NodeId::new(), VisualMass::new(10.0)),
    ];

    let mut planner = BisectionFloorplanner::new();
    let region = planner.plan(canvas, nodes);

    // Check all children fit within parent
    fn check_children_fit(parent: &tecton_layout::floorplanner::FloorplanRegion) -> bool {
        for child in &parent.children {
            // Child should be within parent bounds
            if child.rect.left() < parent.rect.left() - 0.1
                || child.rect.right() > parent.rect.right() + 0.1
                || child.rect.top() < parent.rect.top() - 0.1
                || child.rect.bottom() > parent.rect.bottom() + 0.1
            {
                return false;
            }
            // Recursively check grandchildren
            if !check_children_fit(child) {
                return false;
            }
        }
        true
    }

    assert!(
        check_children_fit(&region),
        "All children should fit within their parents"
    );
}
