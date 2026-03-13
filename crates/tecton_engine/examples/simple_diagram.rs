use tecton_engine::{DiagramDsl, DslParser, Orchestrator};
use tecton_types::error::Result;

fn main() -> Result<()> {
    // Create a simple diagram DSL
    let json = r##"{
        "canvas": {"width": 800, "height": 600},
        "nodes": [
            {
                "label": "Frontend",
                "background": "#e3f2fd",
                "border_color": "#2196f3",
                "ports": [
                    {"name": "api", "side": "east"}
                ]
            },
            {
                "label": "Backend",
                "background": "#f3e5f5",
                "border_color": "#9c27b0",
                "ports": [
                    {"name": "in", "side": "west"},
                    {"name": "db", "side": "east"}
                ]
            },
            {
                "label": "Database",
                "background": "#e8f5e9",
                "border_color": "#4caf50",
                "ports": [
                    {"name": "conn", "side": "west"}
                ]
            }
        ],
        "edges": [
            {"from": "node_1:api", "to": "node_2:in", "color": "#666666"},
            {"from": "node_2:db", "to": "node_3:conn", "color": "#666666"}
        ]
    }"##;

    // Parse the DSL
    let parser = DslParser::new();
    let dsl = parser
        .parse_json(json)
        .map_err(|e| tecton_types::error::TectonError::parse(e))?;

    println!(
        "Parsed {} nodes and {} edges",
        dsl.nodes.len(),
        dsl.edges.len()
    );

    // Convert to layout graph
    let graph = parser
        .to_layout_graph(&dsl)
        .map_err(|e| tecton_types::error::TectonError::parse(e))?;

    println!("Created layout graph with {} nodes", graph.node_count());

    // Create orchestrator and run layout
    let mut orchestrator = Orchestrator::new(graph);
    let (layout, routes) = orchestrator.run()?;

    println!("Layout complete!");
    println!("  Root bounds: {:?}", layout.root.rect);
    println!("  Routes: {}", routes.len());

    // Render to SVG
    let svg = orchestrator.render()?;

    // Save to file
    let output_path = "output.svg";
    std::fs::write(output_path, svg).map_err(|e| tecton_types::error::TectonError::IoError(e))?;

    println!("SVG saved to {}", output_path);

    Ok(())
}
