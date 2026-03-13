//! Tecton DSL Parser
//!
//! A custom domain-specific language for defining architecture diagrams.
//!
//! # Example
//! ```text
//! config {
//!     width: 800,
//!     height: 600
//! }
//!
//! class headerStyle {
//!     fill: "#f0f0f0",
//!     stroke: "#333333",
//!     stroke-width: 2px
//! }
//!
//! Row (gap: 20px) {
//!     Header [label: "API Gateway", class: headerStyle]
//!
//!     Col (weight: 2, gap: 10px) {
//!         Service1 [label: "Auth Service"]
//!         Service2 [label: "User Service"]
//!     }
//! }
//!
//! Header -> Service1
//! Header -> Service2 : "JWT"
//! ```

pub mod ast;
pub mod parser;

pub use ast::*;
pub use parser::parse;

use thiserror::Error;

/// Errors that can occur during DSL parsing
#[derive(Error, Debug)]
pub enum DslError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Parse a Tecton DSL file from a string
pub fn parse_dsl(input: &str) -> Result<TectonFile, DslError> {
    parser::parse(input).map_err(DslError::ParseError)
}

/// Parse a Tecton DSL file from a file path
pub fn parse_dsl_file(path: &str) -> Result<TectonFile, DslError> {
    let content = std::fs::read_to_string(path)?;
    parse_dsl(&content)
}

/// Convert a TectonFile to a string representation (for debugging)
pub fn debug_print(file: &TectonFile) -> String {
    format!("{:#?}", file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_dsl() {
        let dsl = r##"
            config {
                width: 1200,
                height: 800
            }

            class boxStyle {
                fill: "#e3f2fd",
                stroke: "#1976d2",
                stroke-width: 2px
            }

            Row (weight: 1, gap: 30px) {
                Frontend [label: "Frontend App", class: boxStyle]

                Col (weight: 2, gap: 20px) {
                    Backend1 [label: "API Server 1"]
                    Backend2 [label: "API Server 2"]
                }
            }

            Frontend -> Backend1
            Frontend -> Backend2 : "requests"
        "##;

        let file = parse_dsl(dsl).unwrap();
        assert!(file.config.is_some());
        assert_eq!(file.statements.len(), 3); // class, Row, and edge
    }

    #[test]
    fn test_edge_operations() {
        let dsl = r#"
            A -> B
            B => C
            C --> D
        "#;

        let file = parse_dsl(dsl).unwrap();
        assert_eq!(file.statements.len(), 3);
    }

    #[test]
    fn test_nested_containers() {
        let dsl = r#"
            Row {
                Col {
                    Node1 [label: "A"]
                }
                Col {
                    Node2 [label: "B"]
                }
            }
        "#;

        let file = parse_dsl(dsl).unwrap();
        assert_eq!(file.statements.len(), 1);
    }
}
