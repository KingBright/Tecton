//! End-to-end example: From DSL to SVG
//!
//! This example demonstrates the complete workflow:
//! 1. Parse Tecton DSL
//! 2. Convert to LayoutGraph
//! 3. Run layout and routing
//! 4. Render to SVG

use std::fs;

fn main() {
    // Example DSL diagram
    let dsl = r##"
        config {
            width: 1200,
            height: 800
        }

        class headerStyle {
            fill: "#e3f2fd",
            stroke: "#1976d2",
            stroke-width: 2
        }

        class serviceStyle {
            fill: "#f3e5f5",
            stroke: "#7b1fa2",
            stroke-width: 1
        }

        Row (gap: 40) {
            Frontend [label: "Frontend\nReact App", class: headerStyle]

            Col (gap: 20) {
                API [label: "API Gateway", class: headerStyle]

                Row (gap: 30) {
                    Service1 [label: "Auth Service", class: serviceStyle]
                    Service2 [label: "User Service", class: serviceStyle]
                    Service3 [label: "Order Service", class: serviceStyle]
                }
            }

            Database [label: "Database\nPostgreSQL", shape: "Cylinder"]
        }

        Frontend -> API : "HTTP"
        API -> Service1 : "JWT"
        API -> Service2 : "REST"
        API -> Service3 : "REST"
        Service1 -> Database : "SQL"
        Service2 -> Database : "SQL"
        Service3 -> Database : "SQL"
    "##;

    println!("=== Tecton End-to-End Example ===\n");

    // Step 1: Parse DSL
    println!("1. Parsing DSL...");
    let file = match tecton_dsl::parse_dsl(dsl) {
        Ok(f) => {
            println!("   ✓ Parsed successfully");
            println!("   - {} classes defined", f.classes.len());
            println!("   - {} top-level statements", f.statements.len());
            f
        }
        Err(e) => {
            eprintln!("   ✗ Parse error: {}", e);
            return;
        }
    };

    // Step 2: Convert to LayoutGraph
    println!("\n2. Converting to LayoutGraph...");
    let graph = match tecton_engine::tecton_dsl_converter::convert_to_layout_graph(&file) {
        Ok(g) => {
            println!("   ✓ Conversion successful");
            println!("   - {} nodes", g.node_count());
            println!("   - {} edges", g.edge_count());
            g
        }
        Err(e) => {
            eprintln!("   ✗ Conversion error: {}", e);
            return;
        }
    };

    // Step 3: Run orchestrator
    println!("\n3. Running layout and routing...");
    let mut orchestrator = tecton_engine::Orchestrator::new(graph);

    match orchestrator.run() {
        Ok(_) => {
            println!("   ✓ Layout complete");
        }
        Err(e) => {
            eprintln!("   ✗ Layout error: {:?}", e);
            return;
        }
    }

    // Step 4: Render to SVG
    println!("\n4. Rendering to SVG...");
    match orchestrator.render() {
        Ok(svg) => {
            println!("   ✓ Rendered successfully");
            println!("   - SVG size: {} bytes", svg.len());

            // Save to file
            let output_path = "output.svg";
            match fs::write(output_path, svg) {
                Ok(_) => println!("   - Saved to: {}", output_path),
                Err(e) => eprintln!("   ✗ Failed to save: {}", e),
            }
        }
        Err(e) => {
            eprintln!("   ✗ Render error: {:?}", e);
        }
    }

    println!("\n=== Complete ===");
}
