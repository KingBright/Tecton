use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Unique identifier for nodes in the diagram
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(u64);

impl NodeId {
    pub fn new() -> Self {
        Self(next_id())
    }

    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "N{}", self.0)
    }
}

/// Unique identifier for edges (connections) in the diagram
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EdgeId(u64);

impl EdgeId {
    pub fn new() -> Self {
        Self(next_id())
    }

    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{}", self.0)
    }
}

/// Unique identifier for ports (connection points) on nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PortId(u64);

impl PortId {
    pub fn new() -> Self {
        Self(next_id())
    }

    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for PortId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0)
    }
}

/// Reference to a specific port on a specific node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortRef {
    pub node_id: NodeId,
    pub port_id: PortId,
}

impl PortRef {
    pub fn new(node_id: NodeId, port_id: PortId) -> Self {
        Self { node_id, port_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_node_id_unique() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_edge_id_unique() {
        let id1 = EdgeId::new();
        let id2 = EdgeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_port_id_unique() {
        let id1 = PortId::new();
        let id2 = PortId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_node_id_equality() {
        let id = NodeId::from_raw(42);
        let same_id = NodeId::from_raw(42);
        let diff_id = NodeId::from_raw(43);

        assert_eq!(id, same_id);
        assert_ne!(id, diff_id);
    }

    #[test]
    fn test_edge_id_equality() {
        let id = EdgeId::from_raw(42);
        let same_id = EdgeId::from_raw(42);
        let diff_id = EdgeId::from_raw(43);

        assert_eq!(id, same_id);
        assert_ne!(id, diff_id);
    }

    #[test]
    fn test_node_id_hash_consistency() {
        let mut set = HashSet::new();
        let id1 = NodeId::from_raw(100);
        let id2 = NodeId::from_raw(100);
        let id3 = NodeId::from_raw(101);

        set.insert(id1);
        assert!(set.contains(&id2)); // Same ID should be found
        assert!(!set.contains(&id3)); // Different ID should not be found
    }

    #[test]
    fn test_edge_id_hash_consistency() {
        let mut set = HashSet::new();
        let id1 = EdgeId::from_raw(100);
        let id2 = EdgeId::from_raw(100);
        let id3 = EdgeId::from_raw(101);

        set.insert(id1);
        assert!(set.contains(&id2));
        assert!(!set.contains(&id3));
    }

    #[test]
    fn test_port_ref() {
        let node_id = NodeId::from_raw(1);
        let port_id = PortId::from_raw(2);
        let port_ref = PortRef::new(node_id, port_id);

        assert_eq!(port_ref.node_id, node_id);
        assert_eq!(port_ref.port_id, port_id);
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId::from_raw(42);
        assert_eq!(format!("{}", id), "N42");
    }

    #[test]
    fn test_edge_id_display() {
        let id = EdgeId::from_raw(42);
        assert_eq!(format!("{}", id), "E42");
    }

    #[test]
    fn test_port_id_display() {
        let id = PortId::from_raw(42);
        assert_eq!(format!("{}", id), "P42");
    }

    #[test]
    fn test_node_id_ordering() {
        let id1 = NodeId::from_raw(1);
        let id2 = NodeId::from_raw(2);
        assert!(id1 < id2);
    }

    #[test]
    fn test_node_id_default() {
        let id1 = NodeId::default();
        let id2 = NodeId::new();
        assert_ne!(id1, id2); // Each call generates a new unique ID
    }

    #[test]
    fn test_as_u64() {
        let id = NodeId::from_raw(12345);
        assert_eq!(id.as_u64(), 12345);
    }
}
