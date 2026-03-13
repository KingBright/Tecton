//! Pest-based parser for Tecton DSL

use pest::Parser;
use pest_derive::Parser as PestParser;
use std::collections::HashMap;

use crate::ast::*;

#[derive(PestParser)]
#[grammar = "tecton.pest"]
pub struct TectonParser;

pub fn parse(input: &str) -> Result<TectonFile, String> {
    let mut pairs =
        TectonParser::parse(Rule::file, input).map_err(|e| format!("Parse error: {}", e))?;

    let mut file = TectonFile::new();

    // Get the file pair (the first and only pair)
    let file_pair = pairs.next().ok_or("No file pair found")?;

    // Iterate through the inner pairs of file (which should be stmt pairs)
    for pair in file_pair.into_inner() {
        match pair.as_rule() {
            Rule::stmt => {
                let stmt = parse_stmt(pair)?;
                // Handle config and class specially
                match &stmt {
                    Stmt::Config(config) => {
                        file.config = Some(config.clone());
                    }
                    Stmt::Class(class) => {
                        file.classes.push(class.clone());
                    }
                    _ => {
                        file.statements.push(stmt);
                    }
                }
            }
            Rule::EOI => {} // End of input
            _ => {
                // Debug: print unexpected rules
                eprintln!("Unexpected rule in file: {:?}", pair.as_rule());
            }
        }
    }

    Ok(file)
}

fn parse_stmt(pair: pest::iterators::Pair<Rule>) -> Result<Stmt, String> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::container_stmt => parse_container(inner),
        Rule::node_stmt => parse_node(inner),
        Rule::edge_stmt => parse_edge(inner),
        Rule::class_def => parse_class(inner),
        Rule::config_stmt => parse_config_stmt(inner),
        _ => Err(format!("Unexpected rule in stmt: {:?}", inner.as_rule())),
    }
}

fn parse_config_stmt(pair: pest::iterators::Pair<Rule>) -> Result<Stmt, String> {
    let mut properties = HashMap::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::property => {
                let (key, value) = parse_property(inner)?;
                properties.insert(key, value);
            }
            _ => {}
        }
    }

    Ok(Stmt::Config(Config::new(properties)))
}

fn parse_container(pair: pest::iterators::Pair<Rule>) -> Result<Stmt, String> {
    let mut kind = None;
    let mut properties = HashMap::new();
    let mut children = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::container_kind => {
                kind = ContainerKind::from_str(inner.as_str());
            }
            Rule::properties => {
                properties = parse_properties(inner)?;
            }
            Rule::container_body => {
                for stmt_pair in inner.into_inner() {
                    if stmt_pair.as_rule() == Rule::stmt {
                        children.push(parse_stmt(stmt_pair)?);
                    }
                }
            }
            _ => {}
        }
    }

    let kind = kind.ok_or("Missing container kind")?;
    Ok(Stmt::Container(Container::new(kind, properties, children)))
}

fn parse_node(pair: pest::iterators::Pair<Rule>) -> Result<Stmt, String> {
    let mut id = None;
    let mut properties = HashMap::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::node_id => {
                id = Some(inner.as_str().to_string());
            }
            Rule::node_body => {
                for prop_pair in inner.into_inner() {
                    if prop_pair.as_rule() == Rule::property {
                        let (key, value) = parse_property(prop_pair)?;
                        properties.insert(key, value);
                    }
                }
            }
            _ => {}
        }
    }

    let id = id.ok_or("Missing node id")?;
    Ok(Stmt::Node(Node::new(id, properties)))
}

fn parse_edge(pair: pest::iterators::Pair<Rule>) -> Result<Stmt, String> {
    let mut from = None;
    let mut targets = Vec::new();
    let mut label = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::edge_chain => {
                let mut current_node = None;
                let mut last_op = EdgeOp::Arrow;
                for chain_inner in inner.into_inner() {
                    match chain_inner.as_rule() {
                        Rule::node_id => {
                            let node_id = chain_inner.as_str().to_string();
                            if current_node.is_none() {
                                current_node = Some(node_id.clone());
                                from = Some(node_id);
                            } else {
                                targets.push(EdgeTarget::new(node_id, last_op));
                            }
                        }
                        Rule::edge_op => {
                            last_op =
                                EdgeOp::from_str(chain_inner.as_str()).unwrap_or(EdgeOp::Arrow);
                        }
                        _ => {}
                    }
                }
            }
            Rule::edge_label => {
                let label_inner = inner.into_inner().next().unwrap();
                if label_inner.as_rule() == Rule::string {
                    label = Some(parse_string(label_inner)?);
                }
            }
            _ => {}
        }
    }

    let from = from.ok_or("Missing edge source")?;
    Ok(Stmt::Edge(Edge::new(from, targets, label)))
}

fn parse_class(pair: pest::iterators::Pair<Rule>) -> Result<Stmt, String> {
    let mut name = None;
    let mut properties = HashMap::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                name = Some(inner.as_str().to_string());
            }
            Rule::property => {
                let (key, value) = parse_property(inner)?;
                properties.insert(key, value);
            }
            _ => {}
        }
    }

    let name = name.ok_or("Missing class name")?;
    Ok(Stmt::Class(ClassDef::new(name, properties)))
}

fn parse_properties(pair: pest::iterators::Pair<Rule>) -> Result<HashMap<String, Value>, String> {
    let mut properties = HashMap::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::property {
            let (key, value) = parse_property(inner)?;
            properties.insert(key, value);
        }
    }

    Ok(properties)
}

fn parse_property(pair: pest::iterators::Pair<Rule>) -> Result<(String, Value), String> {
    let mut key = None;
    let mut value = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::property_key => {
                key = Some(inner.as_str().to_string());
            }
            Rule::property_value => {
                value = Some(parse_value(inner)?);
            }
            _ => {}
        }
    }

    let key = key.ok_or("Missing property key")?;
    let value = value.ok_or("Missing property value")?;
    Ok((key, value))
}

fn parse_value(pair: pest::iterators::Pair<Rule>) -> Result<Value, String> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::string => {
            let s = parse_string(inner)?;
            Ok(Value::String(s))
        }
        Rule::hex_color => Ok(Value::HexColor(inner.as_str().to_string())),
        Rule::value_with_unit => {
            let s = inner.as_str();
            // Parse number and unit
            let num_str: String = s
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let unit_str: String = s
                .chars()
                .skip_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let num = num_str
                .parse::<f32>()
                .map_err(|e| format!("Invalid number: {}", e))?;
            Ok(Value::WithUnit(num, unit_str.to_string()))
        }
        Rule::number => {
            let num = inner
                .as_str()
                .parse::<f32>()
                .map_err(|e| format!("Invalid number: {}", e))?;
            Ok(Value::Number(num))
        }
        Rule::identifier => Ok(Value::Identifier(inner.as_str().to_string())),
        _ => Err(format!("Unexpected value rule: {:?}", inner.as_rule())),
    }
}

fn parse_string(pair: pest::iterators::Pair<Rule>) -> Result<String, String> {
    let inner = pair.into_inner().next().unwrap();
    if inner.as_rule() == Rule::inner_string {
        Ok(inner.as_str().to_string())
    } else {
        Err("Expected inner_string".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_node() {
        let input = r#"MyNode [label: "Hello"]"#;
        let file = parse(input).unwrap();
        assert_eq!(file.statements.len(), 1);
    }

    #[test]
    fn test_parse_container() {
        let input = r#"
            Row {
                Node1 [label: "A"]
                Node2 [label: "B"]
            }
        "#;
        let file = parse(input).unwrap();
        assert_eq!(file.statements.len(), 1);
    }

    #[test]
    fn test_parse_edge() {
        let input = r#"A -> B -> C : "Flow""#;
        let file = parse(input).unwrap();
        assert_eq!(file.statements.len(), 1);
    }

    #[test]
    fn test_parse_class() {
        let input = r##"
            class myStyle {
                fill: "#ff0000",
                stroke: "#000000"
            }
        "##;
        let file = parse(input).unwrap();
        assert_eq!(file.classes.len(), 1);
        assert_eq!(file.classes[0].name, "myStyle");
    }

    #[test]
    fn test_parse_config() {
        let input = r##"
            config {
                width: 800,
                height: 600
            }
        "##;
        let file = parse(input).unwrap();
        assert!(file.config.is_some());
        let config = file.config.unwrap();
        assert!(config.properties.contains_key("width"));
        assert!(config.properties.contains_key("height"));
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_parse_empty_input() {
        let input = "";
        let file = parse(input).unwrap();
        assert!(file.statements.is_empty());
        assert!(file.classes.is_empty());
        assert!(file.config.is_none());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let input = "   \n\t  \n  ";
        let file = parse(input).unwrap();
        assert!(file.statements.is_empty());
    }

    #[test]
    fn test_parse_node_with_multiple_properties() {
        let input = r##"MyNode [label: "Test", weight: 2.5, fill: "#ff0000"]"##;
        let file = parse(input).unwrap();

        assert_eq!(file.statements.len(), 1);
        if let Stmt::Node(node) = &file.statements[0] {
            assert_eq!(node.id, "MyNode");
            assert_eq!(node.properties.len(), 3);
        } else {
            panic!("Expected node statement");
        }
    }

    #[test]
    fn test_parse_node_empty_body() {
        let input = r#"MyNode []"#;
        let file = parse(input).unwrap();

        assert_eq!(file.statements.len(), 1);
        if let Stmt::Node(node) = &file.statements[0] {
            assert_eq!(node.id, "MyNode");
            assert!(node.properties.is_empty());
        } else {
            panic!("Expected node statement");
        }
    }

    #[test]
    fn test_parse_container_with_properties() {
        let input = r#"Row (gap: 10px, weight: 2) { Node1 [label: "A"] }"#;
        let file = parse(input).unwrap();

        assert_eq!(file.statements.len(), 1);
        if let Stmt::Container(container) = &file.statements[0] {
            assert_eq!(container.kind, ContainerKind::Row);
            assert_eq!(container.properties.len(), 2);
        } else {
            panic!("Expected container statement");
        }
    }

    #[test]
    fn test_parse_column_container() {
        let input = r#"Col { Node1 [label: "A"] }"#;
        let file = parse(input).unwrap();

        if let Stmt::Container(container) = &file.statements[0] {
            assert_eq!(container.kind, ContainerKind::Column);
        } else {
            panic!("Expected container statement");
        }
    }

    #[test]
    fn test_parse_stack_container() {
        let input = r#"Stack { Node1 [label: "A"] }"#;
        let file = parse(input).unwrap();

        if let Stmt::Container(container) = &file.statements[0] {
            assert_eq!(container.kind, ContainerKind::Stack);
        } else {
            panic!("Expected container statement");
        }
    }

    #[test]
    fn test_parse_empty_container() {
        let input = r#"Row { }"#;
        let file = parse(input).unwrap();

        if let Stmt::Container(container) = &file.statements[0] {
            assert!(container.children.is_empty());
        } else {
            panic!("Expected container statement");
        }
    }

    #[test]
    fn test_parse_nested_containers() {
        let input = r#"
            Row {
                Col {
                    Node1 [label: "A"]
                }
            }
        "#;
        let file = parse(input).unwrap();

        if let Stmt::Container(outer) = &file.statements[0] {
            assert_eq!(outer.children.len(), 1);
            if let Stmt::Container(inner) = &outer.children[0] {
                assert_eq!(inner.kind, ContainerKind::Column);
                assert_eq!(inner.children.len(), 1);
            } else {
                panic!("Expected nested container");
            }
        } else {
            panic!("Expected container statement");
        }
    }

    #[test]
    fn test_parse_deep_nesting() {
        let input = r#"
            Row {
                Col {
                    Row {
                        Node1 [label: "Deep"]
                    }
                }
            }
        "#;
        let file = parse(input).unwrap();
        assert_eq!(file.statements.len(), 1);
    }

    #[test]
    fn test_parse_edge_simple() {
        let input = r#"A -> B"#;
        let file = parse(input).unwrap();

        if let Stmt::Edge(edge) = &file.statements[0] {
            assert_eq!(edge.from, "A");
            assert_eq!(edge.to.len(), 1);
            assert_eq!(edge.to[0].node_id, "B");
            assert_eq!(edge.to[0].op, EdgeOp::Arrow);
        } else {
            panic!("Expected edge statement");
        }
    }

    #[test]
    fn test_parse_edge_thick_arrow() {
        let input = r#"A => B"#;
        let file = parse(input).unwrap();

        if let Stmt::Edge(edge) = &file.statements[0] {
            assert_eq!(edge.to[0].op, EdgeOp::ThickArrow);
        } else {
            panic!("Expected edge statement");
        }
    }

    #[test]
    fn test_parse_edge_dashed_arrow() {
        let input = r#"A --> B"#;
        let file = parse(input).unwrap();

        if let Stmt::Edge(edge) = &file.statements[0] {
            assert_eq!(edge.to[0].op, EdgeOp::DashedArrow);
        } else {
            panic!("Expected edge statement");
        }
    }

    #[test]
    fn test_parse_edge_chain() {
        let input = r#"A -> B -> C -> D"#;
        let file = parse(input).unwrap();

        if let Stmt::Edge(edge) = &file.statements[0] {
            assert_eq!(edge.from, "A");
            assert_eq!(edge.to.len(), 3);
        } else {
            panic!("Expected edge statement");
        }
    }

    #[test]
    fn test_parse_edge_with_label() {
        let input = r#"A -> B : "Connection""#;
        let file = parse(input).unwrap();

        if let Stmt::Edge(edge) = &file.statements[0] {
            assert_eq!(edge.label, Some("Connection".to_string()));
        } else {
            panic!("Expected edge statement");
        }
    }

    #[test]
    fn test_parse_class_with_multiple_properties() {
        let input = r##"
            class boxStyle {
                fill: "#ffffff",
                stroke: "#000000",
                stroke-width: 2px,
                border-radius: 5px
            }
        "##;
        let file = parse(input).unwrap();

        assert_eq!(file.classes.len(), 1);
        assert_eq!(file.classes[0].properties.len(), 4);
    }

    #[test]
    fn test_parse_multiple_classes() {
        let input = r##"
            class style1 { fill: "#ff0000" }
            class style2 { fill: "#00ff00" }
        "##;
        let file = parse(input).unwrap();

        assert_eq!(file.classes.len(), 2);
    }

    #[test]
    fn test_parse_value_with_unit_px() {
        let input = r#"Node [width: 100px]"#;
        let file = parse(input).unwrap();

        if let Stmt::Node(node) = &file.statements[0] {
            if let Some(value) = node.properties.get("width") {
                assert_eq!(value.as_number(), Some(100.0));
            } else {
                panic!("Expected width property");
            }
        } else {
            panic!("Expected node statement");
        }
    }

    #[test]
    fn test_parse_value_with_unit_percent() {
        let input = r#"Node [width: 50%]"#;
        let file = parse(input).unwrap();

        if let Stmt::Node(node) = &file.statements[0] {
            if let Some(Value::WithUnit(num, unit)) = node.properties.get("width") {
                assert_eq!(*num, 50.0);
                assert_eq!(unit, "%");
            } else {
                panic!("Expected WithUnit value");
            }
        } else {
            panic!("Expected node statement");
        }
    }

    #[test]
    fn test_parse_hex_color_value() {
        let input = r##"Node [fill: "#ff5733"]"##;
        let file = parse(input).unwrap();

        if let Stmt::Node(node) = &file.statements[0] {
            // Hex color may be parsed as HexColor or String depending on grammar
            let fill = node.properties.get("fill");
            assert!(fill.is_some());
            // Check it's either a HexColor or a String containing the hex value
            let is_hex = matches!(fill, Some(Value::HexColor(_)));
            let is_string_with_hex = matches!(fill, Some(Value::String(s)) if s.starts_with('#'));
            assert!(is_hex || is_string_with_hex);
        } else {
            panic!("Expected node statement");
        }
    }

    #[test]
    fn test_parse_identifier_value() {
        let input = r#"Node [class: myStyle]"#;
        let file = parse(input).unwrap();

        if let Stmt::Node(node) = &file.statements[0] {
            if let Some(Value::Identifier(id)) = node.properties.get("class") {
                assert_eq!(id, "myStyle");
            } else {
                panic!("Expected Identifier value");
            }
        } else {
            panic!("Expected node statement");
        }
    }

    #[test]
    fn test_parse_number_value() {
        let input = r#"Node [weight: 2.5]"#;
        let file = parse(input).unwrap();

        if let Stmt::Node(node) = &file.statements[0] {
            if let Some(value) = node.properties.get("weight") {
                assert_eq!(value.as_number(), Some(2.5));
            } else {
                panic!("Expected weight property");
            }
        } else {
            panic!("Expected node statement");
        }
    }

    #[test]
    fn test_parse_unicode_label() {
        let input = r#"Node [label: "你好世界"]"#;
        let file = parse(input).unwrap();

        if let Stmt::Node(node) = &file.statements[0] {
            if let Some(Value::String(label)) = node.properties.get("label") {
                assert_eq!(label, "你好世界");
            } else {
                panic!("Expected String value");
            }
        } else {
            panic!("Expected node statement");
        }
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let input = r#"Node [label: unclosed"#;
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_multiple_statements() {
        let input = r##"
            config { width: 800 }
            class style { fill: "#fff" }
            Node1 [label: "A"]
            Node2 [label: "B"]
            Node1 -> Node2
        "##;
        let file = parse(input).unwrap();

        assert!(file.config.is_some());
        assert_eq!(file.classes.len(), 1);
        assert_eq!(file.statements.len(), 3); // Node1, Node2, edge
    }

    #[test]
    fn test_parse_tecton_file_builder() {
        let file = TectonFile::new()
            .with_config(Config::new(HashMap::new()))
            .add_class(ClassDef::new("test", HashMap::new()))
            .add_stmt(Stmt::Node(Node::new("Node1", HashMap::new())));

        assert!(file.config.is_some());
        assert_eq!(file.classes.len(), 1);
        assert_eq!(file.statements.len(), 1);
    }
}
