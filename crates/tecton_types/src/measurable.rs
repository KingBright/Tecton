use crate::geometry::Size;
use crate::mass::VisualMass;

/// Trait for objects that have a measurable visual mass
///
/// This trait is used by the layout system to determine the relative
/// importance and space requirements of nodes in the diagram.
pub trait Measurable {
    /// Calculate the visual mass of this object
    fn visual_mass(&self) -> VisualMass;

    /// Get the intrinsic size of this object (if known)
    fn intrinsic_size(&self) -> Option<Size>;

    /// Estimate the minimum size required to display this object
    fn min_size(&self) -> Size;

    /// Estimate the preferred size for this object
    fn preferred_size(&self) -> Size;
}

/// A simple measurable object with fixed mass and size
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedMeasurable {
    pub mass: VisualMass,
    pub size: Size,
}

impl FixedMeasurable {
    pub fn new(mass: VisualMass, size: Size) -> Self {
        Self { mass, size }
    }

    pub fn from_size(size: Size) -> Self {
        let mass = VisualMass::from_size(&size);
        Self::new(mass, size)
    }
}

impl Measurable for FixedMeasurable {
    fn visual_mass(&self) -> VisualMass {
        self.mass
    }

    fn intrinsic_size(&self) -> Option<Size> {
        Some(self.size)
    }

    fn min_size(&self) -> Size {
        self.size
    }

    fn preferred_size(&self) -> Size {
        self.size
    }
}

/// A measurable that delegates to a closure
pub struct FnMeasurable<F, G>
where
    F: Fn() -> VisualMass,
    G: Fn() -> Size,
{
    mass_fn: F,
    size_fn: G,
}

impl<F, G> FnMeasurable<F, G>
where
    F: Fn() -> VisualMass,
    G: Fn() -> Size,
{
    pub fn new(mass_fn: F, size_fn: G) -> Self {
        Self { mass_fn, size_fn }
    }
}

impl<F, G> Measurable for FnMeasurable<F, G>
where
    F: Fn() -> VisualMass,
    G: Fn() -> Size,
{
    fn visual_mass(&self) -> VisualMass {
        (self.mass_fn)()
    }

    fn intrinsic_size(&self) -> Option<Size> {
        Some((self.size_fn)())
    }

    fn min_size(&self) -> Size {
        (self.size_fn)()
    }

    fn preferred_size(&self) -> Size {
        (self.size_fn)()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_measurable() {
        let size = Size::new(100.0, 50.0);
        let measurable = FixedMeasurable::from_size(size);

        assert_eq!(measurable.min_size(), size);
        assert_eq!(measurable.preferred_size(), size);
        assert_eq!(
            measurable.visual_mass().total(),
            (100.0f32 * 50.0f32).sqrt()
        );
    }

    #[test]
    fn test_fn_measurable() {
        let measurable = FnMeasurable::new(|| VisualMass::new(10.0), || Size::new(100.0, 100.0));

        assert_eq!(measurable.visual_mass().total(), 10.0);
        assert_eq!(measurable.min_size(), Size::new(100.0, 100.0));
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_fixed_measurable_new() {
        let mass = VisualMass::new(25.0);
        let size = Size::new(50.0, 50.0);
        let measurable = FixedMeasurable::new(mass, size);

        assert_eq!(measurable.mass, mass);
        assert_eq!(measurable.size, size);
    }

    #[test]
    fn test_fixed_measurable_intrinsic_size() {
        let size = Size::new(100.0, 50.0);
        let measurable = FixedMeasurable::from_size(size);

        assert_eq!(measurable.intrinsic_size(), Some(size));
    }

    #[test]
    fn test_fixed_measurable_visual_mass() {
        let mass = VisualMass::new(42.0).with_content(8.0);
        let measurable = FixedMeasurable::new(mass, Size::new(100.0, 100.0));

        let returned_mass = measurable.visual_mass();
        assert!((returned_mass.total() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_fn_measurable_intrinsic_size() {
        let measurable = FnMeasurable::new(|| VisualMass::new(10.0), || Size::new(200.0, 150.0));

        assert_eq!(measurable.intrinsic_size(), Some(Size::new(200.0, 150.0)));
    }

    #[test]
    fn test_fn_measurable_dynamic_size() {
        let counter = std::cell::Cell::new(0);
        let measurable = FnMeasurable::new(
            || VisualMass::new(10.0),
            || {
                counter.set(counter.get() + 1);
                Size::new(counter.get() as f32 * 100.0, 100.0)
            },
        );

        // Each call should invoke the closure
        let size1 = measurable.min_size();
        let size2 = measurable.preferred_size();

        assert!((size1.width() - 100.0).abs() < 0.001);
        assert!((size2.width() - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_fn_measurable_dynamic_mass() {
        let counter = std::cell::Cell::new(0);
        let measurable = FnMeasurable::new(
            || {
                counter.set(counter.get() + 1);
                VisualMass::new(counter.get() as f32 * 10.0)
            },
            || Size::new(100.0, 100.0),
        );

        let mass1 = measurable.visual_mass();
        let mass2 = measurable.visual_mass();

        assert!((mass1.total() - 10.0).abs() < 0.001);
        assert!((mass2.total() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_measurable_trait_object() {
        let measurable: Box<dyn Measurable> =
            Box::new(FixedMeasurable::from_size(Size::new(100.0, 100.0)));

        let mass = measurable.visual_mass();
        assert!((mass.total() - 100.0).abs() < 0.001);

        let size = measurable.min_size();
        assert!((size.width() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_fixed_measurable_equality() {
        let m1 = FixedMeasurable::new(VisualMass::new(10.0), Size::new(100.0, 50.0));
        let m2 = FixedMeasurable::new(VisualMass::new(10.0), Size::new(100.0, 50.0));

        assert_eq!(m1, m2);
    }

    #[test]
    fn test_fixed_measurable_inequality() {
        let m1 = FixedMeasurable::new(VisualMass::new(10.0), Size::new(100.0, 50.0));
        let m2 = FixedMeasurable::new(VisualMass::new(20.0), Size::new(100.0, 50.0));

        assert_ne!(m1, m2);
    }

    #[test]
    fn test_fixed_measurable_copy() {
        let m1 = FixedMeasurable::from_size(Size::new(100.0, 50.0));
        let m2 = m1; // Copy

        assert_eq!(m1, m2);
    }

    #[test]
    fn test_fixed_measurable_clone() {
        let m1 = FixedMeasurable::from_size(Size::new(100.0, 50.0));
        let m2 = m1.clone();

        assert_eq!(m1, m2);
    }
}
