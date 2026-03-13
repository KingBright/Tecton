use tecton_types::{
    geometry::{Rect, Size},
    mass::VisualMass,
};

/// A spacer element that occupies space in the layout
///
/// Spacers are used to create gaps or reserve space for routing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacer {
    /// Minimum size required
    pub min_size: Size,
    /// Preferred size
    pub preferred_size: Size,
    /// Visual mass (used for weighted layout)
    pub mass: VisualMass,
    /// Whether this spacer is flexible
    pub flexible: bool,
    /// Flex grow factor (if flexible)
    pub flex_grow: f32,
    /// Flex shrink factor (if flexible)
    pub flex_shrink: f32,
}

impl Spacer {
    /// Create a fixed-size spacer
    pub fn fixed(size: Size) -> Self {
        Self {
            min_size: size,
            preferred_size: size,
            mass: VisualMass::from_size(&size),
            flexible: false,
            flex_grow: 0.0,
            flex_shrink: 0.0,
        }
    }

    /// Create a flexible spacer
    pub fn flexible() -> Self {
        Self {
            min_size: Size::new(0.0, 0.0),
            preferred_size: Size::new(0.0, 0.0),
            mass: VisualMass::new(0.0),
            flexible: true,
            flex_grow: 1.0,
            flex_shrink: 1.0,
        }
    }

    /// Create a spacer with minimum size
    pub fn with_min_size(mut self, size: Size) -> Self {
        self.min_size = size;
        self
    }

    /// Create a spacer with preferred size
    pub fn with_preferred_size(mut self, size: Size) -> Self {
        self.preferred_size = size;
        self
    }

    /// Create a spacer with flex grow factor
    pub fn with_flex_grow(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self.flexible = true;
        self
    }

    /// Create a spacer with flex shrink factor
    pub fn with_flex_shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = shrink;
        self.flexible = true;
        self
    }

    /// Create a spacer for routing reservation
    ///
    /// This creates a spacer that reserves space for edge routing
    pub fn for_routing(width: f32, height: f32) -> Self {
        let size = Size::new(width, height);
        Self {
            min_size: size,
            preferred_size: size,
            mass: VisualMass::new(0.0), // Routing spacers don't contribute to visual mass
            flexible: false,
            flex_grow: 0.0,
            flex_shrink: 0.0,
        }
    }

    /// Create a horizontal spacer
    pub fn horizontal(width: f32) -> Self {
        Self::fixed(Size::new(width, 0.0))
    }

    /// Create a vertical spacer
    pub fn vertical(height: f32) -> Self {
        Self::fixed(Size::new(0.0, height))
    }

    /// Get the size this spacer should occupy given available space
    pub fn resolve_size(&self, available: Size) -> Size {
        if !self.flexible {
            return self.preferred_size;
        }

        // For flexible spacers, expand to fill available space
        let width = if self.flex_grow > 0.0 {
            available.width().max(self.min_size.width())
        } else {
            self.preferred_size.width()
        };

        let height = if self.flex_grow > 0.0 {
            available.height().max(self.min_size.height())
        } else {
            self.preferred_size.height()
        };

        Size::new(width, height)
    }

    /// Check if this spacer can fit in the given size
    pub fn can_fit(&self, size: Size) -> bool {
        size.width() >= self.min_size.width() && size.height() >= self.min_size.height()
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::flexible()
    }
}

/// A collection of spacers for complex layout scenarios
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpacerGroup {
    spacers: Vec<Spacer>,
}

impl SpacerGroup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_spacer(mut self, spacer: Spacer) -> Self {
        self.spacers.push(spacer);
        self
    }

    pub fn add_spacer(&mut self, spacer: Spacer) {
        self.spacers.push(spacer);
    }

    /// Calculate the total minimum size required
    pub fn total_min_size(&self, direction: SpacerDirection) -> Size {
        match direction {
            SpacerDirection::Horizontal => {
                let width: f32 = self.spacers.iter().map(|s| s.min_size.width()).sum();
                let height = self
                    .spacers
                    .iter()
                    .map(|s| s.min_size.height())
                    .fold(0.0, f32::max);
                Size::new(width, height)
            }
            SpacerDirection::Vertical => {
                let width = self
                    .spacers
                    .iter()
                    .map(|s| s.min_size.width())
                    .fold(0.0, f32::max);
                let height: f32 = self.spacers.iter().map(|s| s.min_size.height()).sum();
                Size::new(width, height)
            }
        }
    }

    /// Distribute available space among spacers
    pub fn distribute(&self, available: Rect, direction: SpacerDirection) -> Vec<Rect> {
        let total_flex: f32 = self
            .spacers
            .iter()
            .filter(|s| s.flexible)
            .map(|s| s.flex_grow)
            .sum();

        let fixed_size = match direction {
            SpacerDirection::Horizontal => {
                let width: f32 = self
                    .spacers
                    .iter()
                    .filter(|s| !s.flexible)
                    .map(|s| s.preferred_size.width())
                    .sum();
                width
            }
            SpacerDirection::Vertical => {
                let height: f32 = self
                    .spacers
                    .iter()
                    .filter(|s| !s.flexible)
                    .map(|s| s.preferred_size.height())
                    .sum();
                height
            }
        };

        let available_flex_space = match direction {
            SpacerDirection::Horizontal => available.width() - fixed_size,
            SpacerDirection::Vertical => available.height() - fixed_size,
        }
        .max(0.0);

        let mut rects = Vec::new();
        let mut current_pos = 0.0;

        for spacer in &self.spacers {
            let size = if spacer.flexible && total_flex > 0.0 {
                let flex_share = (spacer.flex_grow / total_flex) * available_flex_space;
                match direction {
                    SpacerDirection::Horizontal => {
                        Size::new(flex_share.max(spacer.min_size.width()), available.height())
                    }
                    SpacerDirection::Vertical => {
                        Size::new(available.width(), flex_share.max(spacer.min_size.height()))
                    }
                }
            } else {
                spacer.preferred_size
            };

            let rect = match direction {
                SpacerDirection::Horizontal => Rect::new(
                    available.x() + current_pos,
                    available.y(),
                    size.width(),
                    available.height(),
                ),
                SpacerDirection::Vertical => Rect::new(
                    available.x(),
                    available.y() + current_pos,
                    available.width(),
                    size.height(),
                ),
            };

            rects.push(rect);

            match direction {
                SpacerDirection::Horizontal => current_pos += size.width(),
                SpacerDirection::Vertical => current_pos += size.height(),
            }
        }

        rects
    }
}

/// Direction for spacer distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpacerDirection {
    Horizontal,
    Vertical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_spacer() {
        let spacer = Spacer::fixed(Size::new(100.0, 50.0));
        assert_eq!(
            spacer.resolve_size(Size::new(200.0, 200.0)),
            Size::new(100.0, 50.0)
        );
        assert!(!spacer.flexible);
    }

    #[test]
    fn test_flexible_spacer() {
        let spacer = Spacer::flexible();
        let resolved = spacer.resolve_size(Size::new(200.0, 100.0));
        assert_eq!(resolved.width(), 200.0);
        assert_eq!(resolved.height(), 100.0);
        assert!(spacer.flexible);
    }

    #[test]
    fn test_spacer_group_distribution() {
        let group = SpacerGroup::new()
            .with_spacer(Spacer::fixed(Size::new(50.0, 0.0)))
            .with_spacer(Spacer::flexible())
            .with_spacer(Spacer::fixed(Size::new(50.0, 0.0)));

        let available = Rect::new(0.0, 0.0, 200.0, 100.0);
        let rects = group.distribute(available, SpacerDirection::Horizontal);

        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0].width(), 50.0);
        assert_eq!(rects[1].width(), 100.0); // Flexible spacer gets remaining space
        assert_eq!(rects[2].width(), 50.0);
    }

    #[test]
    fn test_routing_spacer() {
        let spacer = Spacer::for_routing(20.0, 10.0);
        assert_eq!(spacer.min_size, Size::new(20.0, 10.0));
        assert_eq!(spacer.mass.total(), 0.0); // Routing spacers have no visual mass
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_spacer_default() {
        let spacer = Spacer::default();
        assert!(spacer.flexible);
        assert_eq!(spacer.flex_grow, 1.0);
        assert_eq!(spacer.flex_shrink, 1.0);
    }

    #[test]
    fn test_spacer_with_min_size() {
        let spacer = Spacer::flexible().with_min_size(Size::new(50.0, 25.0));

        assert_eq!(spacer.min_size, Size::new(50.0, 25.0));
    }

    #[test]
    fn test_spacer_with_preferred_size() {
        let spacer = Spacer::flexible().with_preferred_size(Size::new(100.0, 50.0));

        assert_eq!(spacer.preferred_size, Size::new(100.0, 50.0));
    }

    #[test]
    fn test_spacer_with_flex_grow() {
        let spacer = Spacer::flexible().with_flex_grow(2.0);

        assert_eq!(spacer.flex_grow, 2.0);
        assert!(spacer.flexible);
    }

    #[test]
    fn test_spacer_with_flex_shrink() {
        let spacer = Spacer::flexible().with_flex_shrink(0.5);

        assert_eq!(spacer.flex_shrink, 0.5);
        assert!(spacer.flexible);
    }

    #[test]
    fn test_spacer_horizontal() {
        let spacer = Spacer::horizontal(50.0);

        assert_eq!(spacer.min_size.width(), 50.0);
        assert_eq!(spacer.min_size.height(), 0.0);
        assert!(!spacer.flexible);
    }

    #[test]
    fn test_spacer_vertical() {
        let spacer = Spacer::vertical(75.0);

        assert_eq!(spacer.min_size.width(), 0.0);
        assert_eq!(spacer.min_size.height(), 75.0);
        assert!(!spacer.flexible);
    }

    #[test]
    fn test_spacer_can_fit() {
        let spacer = Spacer::fixed(Size::new(100.0, 50.0));

        assert!(spacer.can_fit(Size::new(100.0, 50.0)));
        assert!(spacer.can_fit(Size::new(200.0, 100.0)));
        assert!(!spacer.can_fit(Size::new(50.0, 50.0)));
        assert!(!spacer.can_fit(Size::new(100.0, 25.0)));
    }

    #[test]
    fn test_spacer_resolve_size_flexible_no_grow() {
        let mut spacer = Spacer::flexible();
        spacer.flex_grow = 0.0;

        let resolved = spacer.resolve_size(Size::new(200.0, 100.0));

        // Without flex_grow, should use preferred size
        assert_eq!(resolved, spacer.preferred_size);
    }

    #[test]
    fn test_spacer_group_new() {
        let group = SpacerGroup::new();
        assert!(group.spacers.is_empty());
    }

    #[test]
    fn test_spacer_group_default() {
        let group = SpacerGroup::default();
        assert!(group.spacers.is_empty());
    }

    #[test]
    fn test_spacer_group_add_spacer() {
        let mut group = SpacerGroup::new();
        group.add_spacer(Spacer::fixed(Size::new(50.0, 50.0)));

        assert_eq!(group.spacers.len(), 1);
    }

    #[test]
    fn test_spacer_group_total_min_size_horizontal() {
        let group = SpacerGroup::new()
            .with_spacer(Spacer::fixed(Size::new(50.0, 100.0)))
            .with_spacer(Spacer::fixed(Size::new(30.0, 50.0)));

        let total = group.total_min_size(SpacerDirection::Horizontal);

        // Width should sum
        assert_eq!(total.width(), 80.0);
        // Height should be max
        assert_eq!(total.height(), 100.0);
    }

    #[test]
    fn test_spacer_group_total_min_size_vertical() {
        let group = SpacerGroup::new()
            .with_spacer(Spacer::fixed(Size::new(100.0, 50.0)))
            .with_spacer(Spacer::fixed(Size::new(50.0, 30.0)));

        let total = group.total_min_size(SpacerDirection::Vertical);

        // Width should be max
        assert_eq!(total.width(), 100.0);
        // Height should sum
        assert_eq!(total.height(), 80.0);
    }

    #[test]
    fn test_spacer_group_distribute_vertical() {
        let group = SpacerGroup::new()
            .with_spacer(Spacer::fixed(Size::new(0.0, 50.0)))
            .with_spacer(Spacer::flexible())
            .with_spacer(Spacer::fixed(Size::new(0.0, 50.0)));

        let available = Rect::new(0.0, 0.0, 100.0, 200.0);
        let rects = group.distribute(available, SpacerDirection::Vertical);

        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0].height(), 50.0);
        assert_eq!(rects[1].height(), 100.0); // Flexible spacer gets remaining space
        assert_eq!(rects[2].height(), 50.0);
    }

    #[test]
    fn test_spacer_group_distribute_all_flexible() {
        let group = SpacerGroup::new()
            .with_spacer(Spacer::flexible().with_flex_grow(1.0))
            .with_spacer(Spacer::flexible().with_flex_grow(1.0));

        let available = Rect::new(0.0, 0.0, 200.0, 100.0);
        let rects = group.distribute(available, SpacerDirection::Horizontal);

        assert_eq!(rects.len(), 2);
        // Both should split available space equally
        assert_eq!(rects[0].width(), 100.0);
        assert_eq!(rects[1].width(), 100.0);
    }

    #[test]
    fn test_spacer_group_distribute_no_flexible() {
        let group = SpacerGroup::new()
            .with_spacer(Spacer::fixed(Size::new(50.0, 0.0)))
            .with_spacer(Spacer::fixed(Size::new(50.0, 0.0)));

        let available = Rect::new(0.0, 0.0, 200.0, 100.0);
        let rects = group.distribute(available, SpacerDirection::Horizontal);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].width(), 50.0);
        assert_eq!(rects[1].width(), 50.0);
    }

    #[test]
    fn test_spacer_group_empty() {
        let group = SpacerGroup::new();
        let available = Rect::new(0.0, 0.0, 200.0, 100.0);

        let rects = group.distribute(available, SpacerDirection::Horizontal);
        assert!(rects.is_empty());

        let total = group.total_min_size(SpacerDirection::Horizontal);
        assert_eq!(total.width(), 0.0);
        assert_eq!(total.height(), 0.0);
    }

    #[test]
    fn test_negative_spacing_scenario() {
        // Test that negative values don't cause issues
        let spacer = Spacer::fixed(Size::new(-10.0, -10.0));

        // The spacer should still work, just with negative sizes
        assert_eq!(spacer.min_size.width(), -10.0);
    }

    #[test]
    fn test_zero_spacing() {
        let spacer = Spacer::fixed(Size::new(0.0, 0.0));

        assert!(spacer.can_fit(Size::new(0.0, 0.0)));
        assert_eq!(
            spacer.resolve_size(Size::new(100.0, 100.0)),
            Size::new(0.0, 0.0)
        );
    }

    #[test]
    fn test_spacer_equality() {
        let spacer1 = Spacer::fixed(Size::new(100.0, 50.0));
        let spacer2 = Spacer::fixed(Size::new(100.0, 50.0));

        assert_eq!(spacer1, spacer2);
    }

    #[test]
    fn test_spacer_group_equality() {
        let group1 = SpacerGroup::new().with_spacer(Spacer::fixed(Size::new(50.0, 50.0)));
        let group2 = SpacerGroup::new().with_spacer(Spacer::fixed(Size::new(50.0, 50.0)));

        assert_eq!(group1, group2);
    }

    #[test]
    fn test_spacer_direction() {
        assert_ne!(SpacerDirection::Horizontal, SpacerDirection::Vertical);
    }
}
