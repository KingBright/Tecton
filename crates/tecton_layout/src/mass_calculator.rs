use std::collections::HashMap;
use tecton_types::{geometry::Size, id::NodeId, mass::VisualMass, measurable::Measurable};

/// Calculates visual mass for nodes in the diagram
///
/// Visual mass is used by the floorplanner to determine how to split
/// space between nodes. The formula is:
/// W = W_base + W_content + W_connectivity + sum(W_children)
#[derive(Debug, Clone)]
pub struct MassCalculator {
    /// Cache of computed masses
    cache: HashMap<NodeId, VisualMass>,
    /// Base mass per character of text
    content_mass_per_char: f32,
    /// Base mass per connection
    connectivity_mass_per_edge: f32,
}

impl Default for MassCalculator {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            content_mass_per_char: 0.5,
            connectivity_mass_per_edge: 2.0,
        }
    }
}

impl MassCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_content_mass_per_char(mut self, value: f32) -> Self {
        self.content_mass_per_char = value;
        self
    }

    pub fn with_connectivity_mass_per_edge(mut self, value: f32) -> Self {
        self.connectivity_mass_per_edge = value;
        self
    }

    /// Calculate visual mass for a measurable object
    pub fn calculate<T: Measurable>(&mut self, id: NodeId, measurable: &T) -> VisualMass {
        if let Some(&cached) = self.cache.get(&id) {
            return cached;
        }

        let mass = self.compute_mass(measurable);
        self.cache.insert(id, mass);
        mass
    }

    /// Calculate mass from size
    pub fn calculate_from_size(&mut self, id: NodeId, size: &Size) -> VisualMass {
        if let Some(&cached) = self.cache.get(&id) {
            return cached;
        }

        let mass = VisualMass::from_size(size);
        self.cache.insert(id, mass);
        mass
    }

    /// Calculate mass with explicit components
    pub fn calculate_detailed(
        &mut self,
        id: NodeId,
        base: f32,
        text_length: usize,
        connection_count: usize,
        children_mass: VisualMass,
    ) -> VisualMass {
        if let Some(&cached) = self.cache.get(&id) {
            return cached;
        }

        let mass = VisualMass::new(base)
            .with_content(text_length as f32 * self.content_mass_per_char)
            .with_connectivity(connection_count as f32 * self.connectivity_mass_per_edge)
            .with_children(children_mass.total());

        self.cache.insert(id, mass);
        mass
    }

    fn compute_mass<T: Measurable>(&self, measurable: &T) -> VisualMass {
        let base_mass = measurable.visual_mass();
        let size = measurable.preferred_size();

        // If the measurable doesn't have a meaningful mass, derive from size
        if base_mass.total() <= 0.0 {
            VisualMass::from_size(&size)
        } else {
            base_mass
        }
    }

    /// Get cached mass for a node
    pub fn get_mass(&self, id: NodeId) -> Option<VisualMass> {
        self.cache.get(&id).copied()
    }

    /// Insert a mass value into the cache
    pub fn insert_mass(&mut self, id: NodeId, mass: VisualMass) {
        self.cache.insert(id, mass);
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Calculate the total mass of a group of nodes
    pub fn total_mass(&self, ids: &[NodeId]) -> VisualMass {
        ids.iter()
            .filter_map(|id| self.cache.get(id).copied())
            .sum()
    }

    /// Find the optimal split point for a group of nodes
    /// Returns the index where the cumulative mass is closest to half the total
    pub fn find_split_point(&self, ids: &[NodeId]) -> Option<usize> {
        if ids.len() < 2 {
            return None;
        }

        let total_mass: f32 = ids
            .iter()
            .filter_map(|id| self.cache.get(id).map(|m| m.total()))
            .sum();

        let target = total_mass / 2.0;
        let mut cumulative = 0.0;

        for (i, id) in ids.iter().enumerate().take(ids.len() - 1) {
            if let Some(mass) = self.cache.get(id) {
                cumulative += mass.total();
                if cumulative >= target {
                    return Some(i + 1);
                }
            }
        }

        Some(ids.len() / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_from_size() {
        let mut calculator = MassCalculator::new();
        let size = Size::new(100.0, 100.0);
        let id = NodeId::new();

        let mass = calculator.calculate_from_size(id, &size);
        assert_eq!(mass.total(), 100.0); // sqrt(10000)
    }

    #[test]
    fn test_calculate_detailed() {
        let mut calculator = MassCalculator::new();
        let id = NodeId::new();

        let mass = calculator.calculate_detailed(id, 10.0, 20, 2, VisualMass::default());

        assert_eq!(mass.base(), 10.0);
        assert_eq!(mass.content(), 10.0); // 20 * 0.5
        assert_eq!(mass.connectivity(), 4.0); // 2 * 2.0
    }

    #[test]
    fn test_find_split_point() {
        let mut calculator = MassCalculator::new();

        let ids: Vec<NodeId> = (0..4).map(|_| NodeId::new()).collect();

        // Set up masses: 10, 20, 30, 40 (total = 100)
        calculator.insert_mass(ids[0], VisualMass::new(10.0));
        calculator.insert_mass(ids[1], VisualMass::new(20.0));
        calculator.insert_mass(ids[2], VisualMass::new(30.0));
        calculator.insert_mass(ids[3], VisualMass::new(40.0));

        let split = calculator.find_split_point(&ids);
        // Cumulative masses: 10, 30, 60, 100
        // Target is 50, cumulative reaches 60 at index 2, so split at index 3
        assert_eq!(split, Some(3));
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_mass_calculator_default() {
        let calculator = MassCalculator::default();
        assert_eq!(calculator.content_mass_per_char, 0.5);
        assert_eq!(calculator.connectivity_mass_per_edge, 2.0);
        assert!(calculator.cache.is_empty());
    }

    #[test]
    fn test_mass_calculator_new() {
        let calculator = MassCalculator::new();
        assert!(calculator.cache.is_empty());
    }

    #[test]
    fn test_with_content_mass_per_char() {
        let calculator = MassCalculator::new().with_content_mass_per_char(1.0);
        assert_eq!(calculator.content_mass_per_char, 1.0);
    }

    #[test]
    fn test_with_connectivity_mass_per_edge() {
        let calculator = MassCalculator::new().with_connectivity_mass_per_edge(5.0);
        assert_eq!(calculator.connectivity_mass_per_edge, 5.0);
    }

    #[test]
    fn test_calculate_caches_result() {
        let mut calculator = MassCalculator::new();
        let id = NodeId::new();
        let size = Size::new(100.0, 100.0);

        calculator.calculate_from_size(id, &size);
        assert!(calculator.cache.contains_key(&id));

        // Second call should return cached value
        let mass = calculator.calculate_from_size(id, &Size::new(200.0, 200.0));
        assert_eq!(mass.total(), 100.0); // Still the cached value
    }

    #[test]
    fn test_get_mass() {
        let mut calculator = MassCalculator::new();
        let id = NodeId::new();

        calculator.insert_mass(id, VisualMass::new(42.0));

        let mass = calculator.get_mass(id);
        assert!(mass.is_some());
        assert_eq!(mass.unwrap().total(), 42.0);
    }

    #[test]
    fn test_get_mass_missing() {
        let calculator = MassCalculator::new();
        let mass = calculator.get_mass(NodeId::new());
        assert!(mass.is_none());
    }

    #[test]
    fn test_clear_cache() {
        let mut calculator = MassCalculator::new();
        let id = NodeId::new();

        calculator.insert_mass(id, VisualMass::new(10.0));
        assert!(!calculator.cache.is_empty());

        calculator.clear_cache();
        assert!(calculator.cache.is_empty());
    }

    #[test]
    fn test_total_mass_empty() {
        let calculator = MassCalculator::new();
        let ids: Vec<NodeId> = vec![];

        let total = calculator.total_mass(&ids);
        // Empty sum returns default which has base = 1.0
        assert_eq!(total.base(), 1.0);
    }

    #[test]
    fn test_total_mass_with_values() {
        let mut calculator = MassCalculator::new();
        let ids: Vec<NodeId> = (0..3).map(|_| NodeId::new()).collect();

        calculator.insert_mass(ids[0], VisualMass::new(10.0));
        calculator.insert_mass(ids[1], VisualMass::new(20.0));
        calculator.insert_mass(ids[2], VisualMass::new(30.0));

        let total = calculator.total_mass(&ids);
        // Sum uses fold with default (base=1.0), so: 1 + 10 + 20 + 30 = 61
        assert_eq!(total.base(), 61.0);
    }

    #[test]
    fn test_find_split_point_single_node() {
        let calculator = MassCalculator::new();
        let ids = vec![NodeId::new()];

        let split = calculator.find_split_point(&ids);
        assert_eq!(split, None); // Can't split single node
    }

    #[test]
    fn test_find_split_point_empty() {
        let calculator = MassCalculator::new();
        let ids: Vec<NodeId> = vec![];

        let split = calculator.find_split_point(&ids);
        assert_eq!(split, None);
    }

    #[test]
    fn test_find_split_point_two_nodes() {
        let mut calculator = MassCalculator::new();
        let ids: Vec<NodeId> = (0..2).map(|_| NodeId::new()).collect();

        calculator.insert_mass(ids[0], VisualMass::new(10.0));
        calculator.insert_mass(ids[1], VisualMass::new(10.0));

        let split = calculator.find_split_point(&ids);
        assert!(split.is_some());
    }

    #[test]
    fn test_calculate_detailed_with_children() {
        let mut calculator = MassCalculator::new();
        let id = NodeId::new();

        let children_mass = VisualMass::new(50.0).with_content(10.0);
        let mass = calculator.calculate_detailed(id, 10.0, 20, 2, children_mass);

        assert_eq!(mass.base(), 10.0);
        assert_eq!(mass.content(), 10.0); // 20 * 0.5
        assert_eq!(mass.connectivity(), 4.0); // 2 * 2.0
        assert_eq!(mass.children(), 60.0); // children_mass.total()
    }

    #[test]
    fn test_empty_node_list() {
        let calculator = MassCalculator::new();
        let ids: Vec<NodeId> = vec![];

        let split = calculator.find_split_point(&ids);
        assert_eq!(split, None);

        let total = calculator.total_mass(&ids);
        assert_eq!(total.base(), 1.0); // Default
    }

    #[test]
    fn test_super_large_weight() {
        let mut calculator = MassCalculator::new();
        let id = NodeId::new();

        let mass = VisualMass::new(1_000_000.0);
        calculator.insert_mass(id, mass);

        let retrieved = calculator.get_mass(id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().total(), 1_000_000.0);
    }

    #[test]
    fn test_weight_normalization_scenario() {
        let mut calculator = MassCalculator::new();
        let ids: Vec<NodeId> = (0..3).map(|_| NodeId::new()).collect();

        // Very different weights
        calculator.insert_mass(ids[0], VisualMass::new(1.0));
        calculator.insert_mass(ids[1], VisualMass::new(1000.0));
        calculator.insert_mass(ids[2], VisualMass::new(1_000_000.0));

        let split = calculator.find_split_point(&ids);
        // Should still find a valid split point
        assert!(split.is_some());
    }
}
