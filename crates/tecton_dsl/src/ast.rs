//! Abstract Syntax Tree for Tecton DSL

use std::collections::HashMap;

/// A complete Tecton DSL file
#[derive(Debug, Clone, PartialEq)]
pub struct TectonFile {
    pub config: Option<Config>,
    pub classes: Vec<ClassDef>,
    pub statements: Vec<Stmt>,
}

impl TectonFile {
    pub fn new() -> Self {
        Self {
            config: None,
            classes: Vec::new(),
            statements: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn add_class(mut self, class: ClassDef) -> Self {
        self.classes.push(class);
        self
    }

    pub fn add_stmt(mut self, stmt: Stmt) -> Self {
        self.statements.push(stmt);
        self
    }
}

impl Default for TectonFile {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration block
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub properties: HashMap<String, Value>,
}

impl Config {
    pub fn new(properties: HashMap<String, Value>) -> Self {
        Self { properties }
    }
}

/// Class definition (style class)
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    pub name: String,
    pub properties: HashMap<String, Value>,
}

impl ClassDef {
    pub fn new(name: impl Into<String>, properties: HashMap<String, Value>) -> Self {
        Self {
            name: name.into(),
            properties,
        }
    }
}

/// A statement in the DSL
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Container definition (Row, Col, Stack)
    Container(Container),
    /// Node definition
    Node(Node),
    /// Edge definition
    Edge(Edge),
    /// Class definition
    Class(ClassDef),
    /// Config block
    Config(Config),
}

/// Container types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContainerKind {
    Row,
    Column,
    Stack,
}

impl ContainerKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Row" => Some(ContainerKind::Row),
            "Col" | "Column" => Some(ContainerKind::Column),
            "Stack" => Some(ContainerKind::Stack),
            _ => None,
        }
    }
}

/// Container definition
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub kind: ContainerKind,
    pub properties: HashMap<String, Value>,
    pub children: Vec<Stmt>,
}

impl Container {
    pub fn new(
        kind: ContainerKind,
        properties: HashMap<String, Value>,
        children: Vec<Stmt>,
    ) -> Self {
        Self {
            kind,
            properties,
            children,
        }
    }
}

/// Node definition
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub properties: HashMap<String, Value>,
}

impl Node {
    pub fn new(id: impl Into<String>, properties: HashMap<String, Value>) -> Self {
        Self {
            id: id.into(),
            properties,
        }
    }
}

/// Edge definition
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: Vec<EdgeTarget>,
    pub label: Option<String>,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: Vec<EdgeTarget>, label: Option<String>) -> Self {
        Self {
            from: from.into(),
            to,
            label,
        }
    }
}

/// Edge target with operation type
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTarget {
    pub node_id: String,
    pub op: EdgeOp,
}

impl EdgeTarget {
    pub fn new(node_id: impl Into<String>, op: EdgeOp) -> Self {
        Self {
            node_id: node_id.into(),
            op,
        }
    }
}

/// Edge operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeOp {
    /// Simple arrow (->)
    Arrow,
    /// Thick arrow (=>)
    ThickArrow,
    /// Dashed arrow (-->)
    DashedArrow,
}

impl EdgeOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "->" => Some(EdgeOp::Arrow),
            "=>" => Some(EdgeOp::ThickArrow),
            "-->" => Some(EdgeOp::DashedArrow),
            _ => None,
        }
    }
}

/// A value in the DSL
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// String value
    String(String),
    /// Number value
    Number(f32),
    /// Value with unit (e.g., "100px", "50%")
    WithUnit(f32, String),
    /// Hex color
    HexColor(String),
    /// Identifier
    Identifier(String),
}

impl Value {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f32> {
        match self {
            Value::Number(n) => Some(*n),
            Value::WithUnit(n, _) => Some(*n),
            _ => None,
        }
    }

    pub fn as_hex_color(&self) -> Option<&str> {
        match self {
            Value::HexColor(c) => Some(c),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_kind_from_str() {
        assert_eq!(ContainerKind::from_str("Row"), Some(ContainerKind::Row));
        assert_eq!(ContainerKind::from_str("Col"), Some(ContainerKind::Column));
        assert_eq!(
            ContainerKind::from_str("Column"),
            Some(ContainerKind::Column)
        );
        assert_eq!(ContainerKind::from_str("Stack"), Some(ContainerKind::Stack));
        assert_eq!(ContainerKind::from_str("Unknown"), None);
    }

    #[test]
    fn test_edge_op_from_str() {
        assert_eq!(EdgeOp::from_str("->"), Some(EdgeOp::Arrow));
        assert_eq!(EdgeOp::from_str("=>"), Some(EdgeOp::ThickArrow));
        assert_eq!(EdgeOp::from_str("-->"), Some(EdgeOp::DashedArrow));
    }

    #[test]
    fn test_value_conversions() {
        let num = Value::Number(100.0);
        assert_eq!(num.as_number(), Some(100.0));
        assert_eq!(num.as_string(), None);

        let with_unit = Value::WithUnit(50.0, "%".to_string());
        assert_eq!(with_unit.as_number(), Some(50.0));

        let color = Value::HexColor("#ff0000".to_string());
        assert_eq!(color.as_hex_color(), Some("#ff0000"));
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_tecton_file_new() {
        let file = TectonFile::new();

        assert!(file.config.is_none());
        assert!(file.classes.is_empty());
        assert!(file.statements.is_empty());
    }

    #[test]
    fn test_tecton_file_default() {
        let file = TectonFile::default();

        assert!(file.config.is_none());
        assert!(file.classes.is_empty());
        assert!(file.statements.is_empty());
    }

    #[test]
    fn test_tecton_file_with_config() {
        let config = Config::new(HashMap::new());
        let file = TectonFile::new().with_config(config.clone());

        assert!(file.config.is_some());
        assert_eq!(file.config.unwrap(), config);
    }

    #[test]
    fn test_tecton_file_add_class() {
        let class = ClassDef::new("test", HashMap::new());
        let file = TectonFile::new().add_class(class);

        assert_eq!(file.classes.len(), 1);
        assert_eq!(file.classes[0].name, "test");
    }

    #[test]
    fn test_tecton_file_add_stmt() {
        let stmt = Stmt::Node(Node::new("Node1", HashMap::new()));
        let file = TectonFile::new().add_stmt(stmt);

        assert_eq!(file.statements.len(), 1);
    }

    #[test]
    fn test_config_new() {
        let mut props = HashMap::new();
        props.insert("width".to_string(), Value::Number(800.0));

        let config = Config::new(props);

        assert_eq!(config.properties.len(), 1);
        assert!(config.properties.contains_key("width"));
    }

    #[test]
    fn test_class_def_new() {
        let mut props = HashMap::new();
        props.insert("fill".to_string(), Value::HexColor("#fff".to_string()));

        let class = ClassDef::new("myStyle", props);

        assert_eq!(class.name, "myStyle");
        assert_eq!(class.properties.len(), 1);
    }

    #[test]
    fn test_container_new() {
        let container = Container::new(
            ContainerKind::Row,
            HashMap::new(),
            vec![Stmt::Node(Node::new("N1", HashMap::new()))],
        );

        assert_eq!(container.kind, ContainerKind::Row);
        assert_eq!(container.children.len(), 1);
    }

    #[test]
    fn test_node_new() {
        let mut props = HashMap::new();
        props.insert("label".to_string(), Value::String("Test".to_string()));

        let node = Node::new("MyNode", props);

        assert_eq!(node.id, "MyNode");
        assert_eq!(node.properties.len(), 1);
    }

    #[test]
    fn test_edge_new() {
        let targets = vec![
            EdgeTarget::new("B", EdgeOp::Arrow),
            EdgeTarget::new("C", EdgeOp::ThickArrow),
        ];

        let edge = Edge::new("A", targets.clone(), Some("label".to_string()));

        assert_eq!(edge.from, "A");
        assert_eq!(edge.to, targets);
        assert_eq!(edge.label, Some("label".to_string()));
    }

    #[test]
    fn test_edge_target_new() {
        let target = EdgeTarget::new("NodeB", EdgeOp::DashedArrow);

        assert_eq!(target.node_id, "NodeB");
        assert_eq!(target.op, EdgeOp::DashedArrow);
    }

    #[test]
    fn test_all_container_kinds() {
        assert_eq!(ContainerKind::Row, ContainerKind::Row);
        assert_ne!(ContainerKind::Row, ContainerKind::Column);
        assert_ne!(ContainerKind::Column, ContainerKind::Stack);
    }

    #[test]
    fn test_all_edge_ops() {
        assert_eq!(EdgeOp::Arrow, EdgeOp::Arrow);
        assert_ne!(EdgeOp::Arrow, EdgeOp::ThickArrow);
        assert_ne!(EdgeOp::ThickArrow, EdgeOp::DashedArrow);
    }

    #[test]
    fn test_edge_op_from_str_invalid() {
        assert_eq!(EdgeOp::from_str("invalid"), None);
        assert_eq!(EdgeOp::from_str(""), None);
        assert_eq!(EdgeOp::from_str("<-"), None);
    }

    #[test]
    fn test_container_kind_from_str_invalid() {
        assert_eq!(ContainerKind::from_str(""), None);
        assert_eq!(ContainerKind::from_str("row"), None); // case sensitive
        assert_eq!(ContainerKind::from_str("ROW"), None);
    }

    #[test]
    fn test_value_string() {
        let value = Value::String("hello".to_string());

        assert_eq!(value.as_string(), Some("hello"));
        assert_eq!(value.as_number(), None);
        assert_eq!(value.as_hex_color(), None);
    }

    #[test]
    fn test_value_number() {
        let value = Value::Number(42.5);

        assert_eq!(value.as_number(), Some(42.5));
        assert_eq!(value.as_string(), None);
    }

    #[test]
    fn test_value_with_unit() {
        let value = Value::WithUnit(100.0, "px".to_string());

        assert_eq!(value.as_number(), Some(100.0));
        assert_eq!(value.as_string(), None);
    }

    #[test]
    fn test_value_hex_color() {
        let value = Value::HexColor("#ff0000".to_string());

        assert_eq!(value.as_hex_color(), Some("#ff0000"));
        assert_eq!(value.as_number(), None);
    }

    #[test]
    fn test_value_identifier() {
        let value = Value::Identifier("myClass".to_string());

        assert_eq!(value.as_string(), None);
        assert_eq!(value.as_number(), None);
        assert_eq!(value.as_hex_color(), None);
    }

    #[test]
    fn test_stmt_variants() {
        let node_stmt = Stmt::Node(Node::new("N", HashMap::new()));
        let container_stmt =
            Stmt::Container(Container::new(ContainerKind::Row, HashMap::new(), vec![]));
        let edge_stmt = Stmt::Edge(Edge::new("A", vec![], None));
        let class_stmt = Stmt::Class(ClassDef::new("C", HashMap::new()));
        let config_stmt = Stmt::Config(Config::new(HashMap::new()));

        // Just verify they can be constructed
        assert!(matches!(node_stmt, Stmt::Node(_)));
        assert!(matches!(container_stmt, Stmt::Container(_)));
        assert!(matches!(edge_stmt, Stmt::Edge(_)));
        assert!(matches!(class_stmt, Stmt::Class(_)));
        assert!(matches!(config_stmt, Stmt::Config(_)));
    }

    #[test]
    fn test_deeply_nested_structure() {
        // Create a deeply nested structure to test nesting depth
        let inner_node = Stmt::Node(Node::new("Inner", HashMap::new()));
        let inner_container = Stmt::Container(Container::new(
            ContainerKind::Column,
            HashMap::new(),
            vec![inner_node],
        ));
        let middle_container = Stmt::Container(Container::new(
            ContainerKind::Row,
            HashMap::new(),
            vec![inner_container],
        ));
        let outer_container = Stmt::Container(Container::new(
            ContainerKind::Stack,
            HashMap::new(),
            vec![middle_container],
        ));

        if let Stmt::Container(outer) = outer_container {
            assert_eq!(outer.kind, ContainerKind::Stack);
            assert_eq!(outer.children.len(), 1);
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn test_value_equality() {
        let v1 = Value::Number(10.0);
        let v2 = Value::Number(10.0);
        let v3 = Value::Number(20.0);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_node_equality() {
        let n1 = Node::new("A", HashMap::new());
        let n2 = Node::new("A", HashMap::new());
        let n3 = Node::new("B", HashMap::new());

        assert_eq!(n1, n2);
        assert_ne!(n1, n3);
    }

    #[test]
    fn test_edge_equality() {
        let e1 = Edge::new("A", vec![], None);
        let e2 = Edge::new("A", vec![], None);
        let e3 = Edge::new("B", vec![], None);

        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }
}
