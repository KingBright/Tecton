use serde::{Deserialize, Serialize};

use crate::geometry::PortSide;
// Re-export id types for convenience
pub use crate::id::{EdgeId, PortId};
use crate::style::Color;

/// Type of edge (connection)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Standard connection
    #[default]
    Default,
    /// Dashed line (for indirect relationships)
    Dashed,
    /// Dotted line (for weak relationships)
    Dotted,
    /// Thick line (for strong relationships)
    Bold,
    /// Bidirectional connection
    Bidirectional,
}

impl EdgeKind {
    pub fn stroke_dasharray(&self) -> Option<String> {
        match self {
            EdgeKind::Default | EdgeKind::Bold | EdgeKind::Bidirectional => None,
            EdgeKind::Dashed => Some("5,5".to_string()),
            EdgeKind::Dotted => Some("2,2".to_string()),
        }
    }

    pub fn stroke_width(&self) -> f32 {
        match self {
            EdgeKind::Bold => 3.0,
            _ => 1.0,
        }
    }
}

/// An edge connecting two nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub source_port: PortId,
    pub target_port: PortId,
    pub kind: EdgeKind,
    #[serde(default = "default_edge_color")]
    pub color: Color,
    #[serde(default)]
    pub label: Option<String>,
}

fn default_edge_color() -> Color {
    Color::GRAY_500
}

impl Edge {
    pub fn new(id: EdgeId, source_port: PortId, target_port: PortId) -> Self {
        Self {
            id,
            source_port,
            target_port,
            kind: EdgeKind::Default,
            color: default_edge_color(),
            label: None,
        }
    }

    pub fn with_kind(mut self, kind: EdgeKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// A point along an edge route
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoutePoint {
    /// Move to this point (start of a segment)
    Move,
    /// Line to this point
    Line,
}

/// A segment of an edge route
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteSegment {
    pub point_type: RoutePoint,
    pub x: f32,
    pub y: f32,
}

impl RouteSegment {
    pub fn move_to(x: f32, y: f32) -> Self {
        Self {
            point_type: RoutePoint::Move,
            x,
            y,
        }
    }

    pub fn line_to(x: f32, y: f32) -> Self {
        Self {
            point_type: RoutePoint::Line,
            x,
            y,
        }
    }
}

/// Preferred routing hint for an edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RoutingHint {
    /// No preference
    #[default]
    Any,
    /// Prefer routing on the top side
    Top,
    /// Prefer routing on the bottom side
    Bottom,
    /// Prefer routing on the left side
    Left,
    /// Prefer routing on the right side
    Right,
    /// Prefer horizontal routing
    Horizontal,
    /// Prefer vertical routing
    Vertical,
}

impl RoutingHint {
    pub fn preferred_exit_side(&self) -> Option<PortSide> {
        match self {
            RoutingHint::Top => Some(PortSide::North),
            RoutingHint::Bottom => Some(PortSide::South),
            RoutingHint::Left => Some(PortSide::West),
            RoutingHint::Right => Some(PortSide::East),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_kind_stroke() {
        assert_eq!(EdgeKind::Default.stroke_dasharray(), None);
        assert_eq!(EdgeKind::Dashed.stroke_dasharray(), Some("5,5".to_string()));
        assert_eq!(EdgeKind::Dotted.stroke_dasharray(), Some("2,2".to_string()));
    }

    #[test]
    fn test_routing_hint() {
        assert_eq!(
            RoutingHint::Top.preferred_exit_side(),
            Some(PortSide::North)
        );
        assert_eq!(
            RoutingHint::Left.preferred_exit_side(),
            Some(PortSide::West)
        );
        assert_eq!(RoutingHint::Any.preferred_exit_side(), None);
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_all_edge_kinds() {
        // Test all EdgeKind variants exist and work
        let kinds = [
            EdgeKind::Default,
            EdgeKind::Dashed,
            EdgeKind::Dotted,
            EdgeKind::Bold,
            EdgeKind::Bidirectional,
        ];

        for kind in kinds {
            // Each kind should have a stroke width
            let width = kind.stroke_width();
            assert!(width > 0.0);
        }
    }

    #[test]
    fn test_edge_kind_default() {
        assert_eq!(EdgeKind::default(), EdgeKind::Default);
    }

    #[test]
    fn test_edge_kind_stroke_width() {
        assert!((EdgeKind::Default.stroke_width() - 1.0).abs() < 0.001);
        assert!((EdgeKind::Bold.stroke_width() - 3.0).abs() < 0.001);
        assert!((EdgeKind::Dashed.stroke_width() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_edge_kind_bold_no_dash() {
        assert_eq!(EdgeKind::Bold.stroke_dasharray(), None);
    }

    #[test]
    fn test_edge_kind_bidirectional_no_dash() {
        assert_eq!(EdgeKind::Bidirectional.stroke_dasharray(), None);
    }

    #[test]
    fn test_edge_creation() {
        let edge_id = EdgeId::new();
        let source_port = PortId::new();
        let target_port = PortId::new();

        let edge = Edge::new(edge_id, source_port, target_port);

        assert_eq!(edge.id, edge_id);
        assert_eq!(edge.source_port, source_port);
        assert_eq!(edge.target_port, target_port);
        assert_eq!(edge.kind, EdgeKind::Default);
        assert!(edge.label.is_none());
    }

    #[test]
    fn test_edge_with_kind() {
        let edge =
            Edge::new(EdgeId::new(), PortId::new(), PortId::new()).with_kind(EdgeKind::Dashed);

        assert_eq!(edge.kind, EdgeKind::Dashed);
    }

    #[test]
    fn test_edge_with_color() {
        let color = Color::RED;
        let edge = Edge::new(EdgeId::new(), PortId::new(), PortId::new()).with_color(color);

        assert_eq!(edge.color, color);
    }

    #[test]
    fn test_edge_with_label() {
        let edge = Edge::new(EdgeId::new(), PortId::new(), PortId::new()).with_label("test label");

        assert_eq!(edge.label, Some("test label".to_string()));
    }

    #[test]
    fn test_edge_default_color() {
        let edge = Edge::new(EdgeId::new(), PortId::new(), PortId::new());
        assert_eq!(edge.color, Color::GRAY_500);
    }

    #[test]
    fn test_all_routing_hints() {
        let hints = [
            RoutingHint::Any,
            RoutingHint::Top,
            RoutingHint::Bottom,
            RoutingHint::Left,
            RoutingHint::Right,
            RoutingHint::Horizontal,
            RoutingHint::Vertical,
        ];

        for hint in hints {
            // Each hint should be valid
            let _ = hint.preferred_exit_side();
        }
    }

    #[test]
    fn test_routing_hint_default() {
        assert_eq!(RoutingHint::default(), RoutingHint::Any);
    }

    #[test]
    fn test_routing_hint_all_sides() {
        assert_eq!(
            RoutingHint::Top.preferred_exit_side(),
            Some(PortSide::North)
        );
        assert_eq!(
            RoutingHint::Bottom.preferred_exit_side(),
            Some(PortSide::South)
        );
        assert_eq!(
            RoutingHint::Left.preferred_exit_side(),
            Some(PortSide::West)
        );
        assert_eq!(
            RoutingHint::Right.preferred_exit_side(),
            Some(PortSide::East)
        );
    }

    #[test]
    fn test_routing_hint_no_side_preference() {
        assert_eq!(RoutingHint::Any.preferred_exit_side(), None);
        assert_eq!(RoutingHint::Horizontal.preferred_exit_side(), None);
        assert_eq!(RoutingHint::Vertical.preferred_exit_side(), None);
    }

    #[test]
    fn test_route_segment_move_to() {
        let segment = RouteSegment::move_to(10.0, 20.0);

        assert_eq!(segment.point_type, RoutePoint::Move);
        assert!((segment.x - 10.0).abs() < 0.001);
        assert!((segment.y - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_route_segment_line_to() {
        let segment = RouteSegment::line_to(30.0, 40.0);

        assert_eq!(segment.point_type, RoutePoint::Line);
        assert!((segment.x - 30.0).abs() < 0.001);
        assert!((segment.y - 40.0).abs() < 0.001);
    }

    #[test]
    fn test_edge_builder_pattern() {
        let edge = Edge::new(EdgeId::new(), PortId::new(), PortId::new())
            .with_kind(EdgeKind::Bold)
            .with_color(Color::RED)
            .with_label("important edge");

        assert_eq!(edge.kind, EdgeKind::Bold);
        assert_eq!(edge.color, Color::RED);
        assert_eq!(edge.label, Some("important edge".to_string()));
    }
}
