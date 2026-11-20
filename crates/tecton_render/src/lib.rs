//! Tecton Render - Rendering layer for Tecton
//!
//! This crate provides:
//! - Backend trait for pluggable rendering
//! - SVG backend for vector graphics output
//! - Debug backend for grid visualization

pub mod backend;
pub mod fixed_scene;
mod legend;
pub mod svg_backend;

pub use backend::{Backend, DebugBackend};
pub use legend::LegendEntry;
pub use svg_backend::SvgBackend;

use tecton_layout::layout_context::LayoutResult;
use tecton_routing::route::RouteSet;
use tecton_types::error::Result;

/// High-level rendering API
pub struct Renderer {
    backend: Box<dyn Backend>,
}

impl Renderer {
    /// Create a new SVG renderer with default dimensions
    pub fn svg(width: f32, height: f32) -> Self {
        Self {
            backend: Box::new(SvgBackend::new(width, height)),
        }
    }

    /// Render a diagram
    pub fn render(&mut self, layout: &LayoutResult, routes: Option<&RouteSet>) -> Result<String> {
        self.backend.render(layout, routes)?;
        Ok(self.backend.output().unwrap_or_default())
    }

    /// Save the rendered output to a file
    pub fn save(&self, path: &str) -> Result<()> {
        self.backend.save(path)
    }

    /// Get the raw output from the backend
    pub fn output(&self) -> Option<String> {
        self.backend.output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::svg(800.0, 600.0);
        // Before rendering, output returns empty content (not None)
        assert!(renderer.output().is_some());
        assert!(renderer.output().unwrap().is_empty());
    }
}
