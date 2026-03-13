//! Tecton Layout - Layout calculation layer for Tecton
//!
//! This crate provides:
//! - Weighted recursive bisection floorplanning
//! - Taffy (Flexbox) integration for local layout
//! - Spacer management for routing reservation

pub mod floorplanner;
pub mod layout_context;
pub mod mass_calculator;
pub mod spacer;
pub mod taffy_adapter;

pub use floorplanner::{BisectionFloorplanner, FloorplanRegion, GridFloorplanner, SplitDirection};
pub use layout_context::{LayoutNode, LayoutResult};
pub use mass_calculator::MassCalculator;
pub use spacer::{Spacer, SpacerDirection, SpacerGroup};
pub use taffy_adapter::{layout_container, TaffyAdapter};

use tecton_types::{geometry::Rect, id::NodeId, mass::VisualMass};

/// Main layout engine that coordinates floorplanning and flexbox layout
pub struct LayoutEngine {
    floorplanner: BisectionFloorplanner,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            floorplanner: BisectionFloorplanner::new(),
        }
    }

    /// Perform high-level floorplanning
    ///
    /// This divides the canvas into regions based on node masses
    pub fn floorplan(&mut self, bounds: Rect, nodes: Vec<(NodeId, VisualMass)>) -> FloorplanRegion {
        self.floorplanner.plan(bounds, nodes)
    }

    /// Get a reference to the mass calculator
    pub fn mass_calculator(&self) -> &MassCalculator {
        self.floorplanner.calculator()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_engine() {
        let mut engine = LayoutEngine::new();

        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let nodes: Vec<(NodeId, VisualMass)> = vec![
            (NodeId::new(), VisualMass::new(10.0)),
            (NodeId::new(), VisualMass::new(10.0)),
            (NodeId::new(), VisualMass::new(10.0)),
            (NodeId::new(), VisualMass::new(10.0)),
        ];

        let region = engine.floorplan(bounds, nodes);
        assert_eq!(region.rect, bounds);
    }
}
