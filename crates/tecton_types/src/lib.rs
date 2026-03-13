//! Tecton Types - Base types for the Tecton architecture diagram rendering engine
//!
//! This crate provides the foundational types used throughout the Tecton system:
//! - ID types for nodes, edges, and ports
//! - Geometry types (Point, Size, Rect)
//! - Visual mass for weighted layout
//! - Styling properties
//! - Edge and connection types
//! - Measurable trait for layout calculations
//! - Error types

pub mod edge;
pub mod error;
pub mod geometry;
pub mod id;
pub mod mass;
pub mod measurable;
pub mod shape;
pub mod style;

// Re-export commonly used types
pub use edge::{Edge, EdgeId, EdgeKind, PortId, RoutePoint, RouteSegment, RoutingHint};
pub use error::{Result, TectonError};
pub use geometry::{Point, Port, PortSide, Rect, Size};
pub use id::{NodeId, PortRef};
pub use mass::VisualMass;
pub use measurable::{FixedMeasurable, Measurable};
pub use shape::{
    CircleShape, CylinderShape, DiamondShape, Direction, ParallelogramShape, RectangleShape,
    ShapeFactory, ShapeGeometry, ShapeType,
};
pub use style::{Color, FlexDirection, Margin, Padding, Style};
