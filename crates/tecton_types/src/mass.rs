use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use crate::geometry::Size;

/// Visual mass represents the "weight" of a node in the diagram
/// Used for weighted recursive bisection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VisualMass {
    /// Base weight of the node itself
    pub base: OrderedFloat<f32>,
    /// Weight from content (text, labels, etc.)
    pub content: OrderedFloat<f32>,
    /// Weight from connectivity (ports, edges)
    pub connectivity: OrderedFloat<f32>,
    /// Weight from children (for container nodes)
    pub children: OrderedFloat<f32>,
}

impl VisualMass {
    pub fn new(base: f32) -> Self {
        Self {
            base: OrderedFloat(base),
            content: OrderedFloat(0.0),
            connectivity: OrderedFloat(0.0),
            children: OrderedFloat(0.0),
        }
    }

    pub fn with_content(mut self, content: f32) -> Self {
        self.content = OrderedFloat(content);
        self
    }

    pub fn with_connectivity(mut self, connectivity: f32) -> Self {
        self.connectivity = OrderedFloat(connectivity);
        self
    }

    pub fn with_children(mut self, children: f32) -> Self {
        self.children = OrderedFloat(children);
        self
    }

    pub fn base(&self) -> f32 {
        self.base.into_inner()
    }

    pub fn content(&self) -> f32 {
        self.content.into_inner()
    }

    pub fn connectivity(&self) -> f32 {
        self.connectivity.into_inner()
    }

    pub fn children(&self) -> f32 {
        self.children.into_inner()
    }

    /// Total visual mass: W = W_base + W_content + W_connectivity + sum(W_children)
    pub fn total(&self) -> f32 {
        self.base() + self.content() + self.connectivity() + self.children()
    }

    /// Calculate mass from a desired size (area-based)
    pub fn from_size(size: &Size) -> Self {
        Self::new(size.area().sqrt())
    }

    /// Calculate the ideal size for this mass given an aspect ratio
    pub fn to_size(&self, aspect_ratio: f32) -> Size {
        let total = self.total();
        let width = (total * aspect_ratio).sqrt();
        let height = total / width;
        Size::new(width, height)
    }
}

impl Default for VisualMass {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl std::ops::Add for VisualMass {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            base: OrderedFloat(self.base() + rhs.base()),
            content: OrderedFloat(self.content() + rhs.content()),
            connectivity: OrderedFloat(self.connectivity() + rhs.connectivity()),
            children: OrderedFloat(self.children() + rhs.children()),
        }
    }
}

impl std::iter::Sum for VisualMass {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, mass| acc + mass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_mass_total() {
        let mass = VisualMass::new(10.0)
            .with_content(5.0)
            .with_connectivity(3.0)
            .with_children(2.0);

        assert_eq!(mass.total(), 20.0);
    }

    #[test]
    fn test_visual_mass_add() {
        let mass1 = VisualMass::new(10.0).with_content(5.0);
        let mass2 = VisualMass::new(5.0).with_connectivity(3.0);

        let sum = mass1 + mass2;
        assert_eq!(sum.base(), 15.0);
        assert_eq!(sum.content(), 5.0);
        assert_eq!(sum.connectivity(), 3.0);
    }

    #[test]
    fn test_from_size() {
        let size = Size::new(100.0, 100.0);
        let mass = VisualMass::from_size(&size);
        assert_eq!(mass.total(), 100.0); // sqrt(10000)
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_zero_mass() {
        let mass = VisualMass::new(0.0);
        assert_eq!(mass.total(), 0.0);
        assert_eq!(mass.base(), 0.0);
    }

    #[test]
    fn test_mass_default() {
        let mass = VisualMass::default();
        assert_eq!(mass.base(), 1.0); // Default base is 1.0
        assert_eq!(mass.content(), 0.0);
        assert_eq!(mass.connectivity(), 0.0);
        assert_eq!(mass.children(), 0.0);
    }

    #[test]
    fn test_mass_to_size() {
        let mass = VisualMass::new(100.0);
        let size = mass.to_size(1.0); // Square aspect ratio

        assert!((size.width() - 10.0).abs() < 0.001);
        assert!((size.height() - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_mass_to_size_wide() {
        let mass = VisualMass::new(100.0);
        let size = mass.to_size(4.0); // Wide aspect ratio

        assert!((size.width() - 20.0).abs() < 0.001);
        assert!((size.height() - 5.0).abs() < 0.001);
        assert!((size.aspect_ratio() - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_mass_with_all_components() {
        let mass = VisualMass::new(5.0)
            .with_content(10.0)
            .with_connectivity(15.0)
            .with_children(20.0);

        assert_eq!(mass.base(), 5.0);
        assert_eq!(mass.content(), 10.0);
        assert_eq!(mass.connectivity(), 15.0);
        assert_eq!(mass.children(), 20.0);
        assert_eq!(mass.total(), 50.0);
    }

    #[test]
    fn test_mass_sum_iterator() {
        let masses = vec![
            VisualMass::new(10.0),
            VisualMass::new(20.0),
            VisualMass::new(30.0),
        ];

        let sum: VisualMass = masses.into_iter().sum();
        // Sum uses fold with default (base=1.0), so sum is 1 + 10 + 20 + 30 = 61
        assert_eq!(sum.base(), 61.0);
    }

    #[test]
    fn test_from_size_square() {
        let size = Size::new(10.0, 10.0);
        let mass = VisualMass::from_size(&size);

        // sqrt(area) = sqrt(100) = 10
        assert!((mass.total() - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_from_size_rectangular() {
        let size = Size::new(200.0, 50.0);
        let mass = VisualMass::from_size(&size);

        // sqrt(area) = sqrt(10000) = 100
        assert!((mass.total() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_mass_ratio() {
        let mass1 = VisualMass::new(100.0);
        let mass2 = VisualMass::new(50.0);

        let ratio = mass1.total() / mass2.total();
        assert!((ratio - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_mass_empty_sum() {
        let masses: Vec<VisualMass> = vec![];
        let sum: VisualMass = masses.into_iter().sum();

        // Empty sum should return default (base = 1.0)
        assert_eq!(sum.base(), 1.0);
    }
}
