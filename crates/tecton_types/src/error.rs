use thiserror::Error;

/// Main error type for Tecton
#[derive(Error, Debug)]
pub enum TectonError {
    /// Invalid input or configuration
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Layout calculation failed
    #[error("Layout failed: {0}")]
    LayoutError(String),

    /// Routing failed
    #[error("Routing failed: {0}")]
    RoutingError(String),

    /// Rendering failed
    #[error("Rendering failed: {0}")]
    RenderError(String),

    /// Parsing failed
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Node not found
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Edge not found
    #[error("Edge not found: {0}")]
    EdgeNotFound(String),

    /// Port not found
    #[error("Port not found: {0}")]
    PortNotFound(String),

    /// Cycle detected in graph
    #[error("Cycle detected: {0}")]
    CycleDetected(String),

    /// Maximum iterations exceeded
    #[error("Maximum iterations exceeded: {0}")]
    MaxIterationsExceeded(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

/// Result type alias for Tecton
pub type Result<T> = std::result::Result<T, TectonError>;

impl TectonError {
    /// Create a new invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create a new layout error
    pub fn layout(msg: impl Into<String>) -> Self {
        Self::LayoutError(msg.into())
    }

    /// Create a new routing error
    pub fn routing(msg: impl Into<String>) -> Self {
        Self::RoutingError(msg.into())
    }

    /// Create a new render error
    pub fn render(msg: impl Into<String>) -> Self {
        Self::RenderError(msg.into())
    }

    /// Create a new parse error
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::ParseError(msg.into())
    }

    /// Create a new node not found error
    pub fn node_not_found(id: impl Into<String>) -> Self {
        Self::NodeNotFound(id.into())
    }

    /// Create a new edge not found error
    pub fn edge_not_found(id: impl Into<String>) -> Self {
        Self::EdgeNotFound(id.into())
    }

    /// Create a new port not found error
    pub fn port_not_found(id: impl Into<String>) -> Self {
        Self::PortNotFound(id.into())
    }

    /// Create a new cycle detected error
    pub fn cycle_detected(msg: impl Into<String>) -> Self {
        Self::CycleDetected(msg.into())
    }

    /// Create a new max iterations exceeded error
    pub fn max_iterations(msg: impl Into<String>) -> Self {
        Self::MaxIterationsExceeded(msg.into())
    }
}

impl From<serde_json::Error> for TectonError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<serde_yaml::Error> for TectonError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TectonError::invalid_input("test error");
        assert_eq!(err.to_string(), "Invalid input: test error");

        let err = TectonError::node_not_found("node-1");
        assert_eq!(err.to_string(), "Node not found: node-1");
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_all_error_types() {
        // Test that all error variants can be created and displayed
        let errors: Vec<TectonError> = vec![
            TectonError::invalid_input("test"),
            TectonError::layout("test"),
            TectonError::routing("test"),
            TectonError::render("test"),
            TectonError::parse("test"),
            TectonError::node_not_found("test"),
            TectonError::edge_not_found("test"),
            TectonError::port_not_found("test"),
            TectonError::cycle_detected("test"),
            TectonError::max_iterations("test"),
            TectonError::SerializationError("test".to_string()),
            TectonError::Other("test".to_string()),
        ];

        for err in errors {
            // Each error should have a non-empty display
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn test_invalid_input() {
        let err = TectonError::invalid_input("bad value");
        assert!(err.to_string().contains("Invalid input"));
        assert!(err.to_string().contains("bad value"));
    }

    #[test]
    fn test_layout_error() {
        let err = TectonError::layout("overflow");
        assert!(err.to_string().contains("Layout failed"));
        assert!(err.to_string().contains("overflow"));
    }

    #[test]
    fn test_routing_error() {
        let err = TectonError::routing("no path");
        assert!(err.to_string().contains("Routing failed"));
        assert!(err.to_string().contains("no path"));
    }

    #[test]
    fn test_render_error() {
        let err = TectonError::render("canvas error");
        assert!(err.to_string().contains("Rendering failed"));
        assert!(err.to_string().contains("canvas error"));
    }

    #[test]
    fn test_parse_error() {
        let err = TectonError::parse("invalid syntax");
        assert!(err.to_string().contains("Parse error"));
        assert!(err.to_string().contains("invalid syntax"));
    }

    #[test]
    fn test_node_not_found() {
        let err = TectonError::node_not_found("N123");
        assert!(err.to_string().contains("Node not found"));
        assert!(err.to_string().contains("N123"));
    }

    #[test]
    fn test_edge_not_found() {
        let err = TectonError::edge_not_found("E456");
        assert!(err.to_string().contains("Edge not found"));
        assert!(err.to_string().contains("E456"));
    }

    #[test]
    fn test_port_not_found() {
        let err = TectonError::port_not_found("P789");
        assert!(err.to_string().contains("Port not found"));
        assert!(err.to_string().contains("P789"));
    }

    #[test]
    fn test_cycle_detected() {
        let err = TectonError::cycle_detected("A -> B -> A");
        assert!(err.to_string().contains("Cycle detected"));
        assert!(err.to_string().contains("A -> B -> A"));
    }

    #[test]
    fn test_max_iterations() {
        let err = TectonError::max_iterations("100");
        assert!(err.to_string().contains("Maximum iterations exceeded"));
        assert!(err.to_string().contains("100"));
    }

    #[test]
    fn test_serialization_error() {
        let err = TectonError::SerializationError("JSON parse failed".to_string());
        assert!(err.to_string().contains("Serialization error"));
        assert!(err.to_string().contains("JSON parse failed"));
    }

    #[test]
    fn test_other_error() {
        let err = TectonError::Other("something went wrong".to_string());
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let tecton_err: TectonError = io_err.into();

        assert!(tecton_err.to_string().contains("IO error"));
        assert!(tecton_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let tecton_err: TectonError = json_err.into();

        assert!(tecton_err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_yaml_error_conversion() {
        let yaml_err = serde_yaml::from_str::<i32>("not: a: number").unwrap_err();
        let tecton_err: TectonError = yaml_err.into();

        assert!(tecton_err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(TectonError::invalid_input("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_error_chain_debug() {
        let err = TectonError::layout("nested error");
        // Debug should work without panic
        let debug_str = format!("{:?}", err);
        assert!(!debug_str.is_empty());
    }
}
