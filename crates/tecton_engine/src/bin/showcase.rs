//! Showcase Generator - Batch generate example diagrams
//!
//! This binary generates SVG output for all showcase examples
//! and creates an HTML gallery page for visual inspection.

use std::fs;
use std::path::Path;
use std::time::Instant;

const EXAMPLES_DIR: &str = "showcase/examples";
const OUTPUTS_DIR: &str = "showcase/outputs";

/// Example metadata
struct Example {
    name: &'static str,
    description: &'static str,
    features: &'static [&'static str],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(OUTPUTS_DIR)?;

    let examples = vec![
        Example {
            name: "01_simple_node",
            description: "单个节点 - 最基础的用例，展示基本渲染功能",
            features: &["基础节点", "标签渲染"],
        },
        Example {
            name: "02_row_layout",
            description: "水平布局 - 展示 Row 容器和 Flexbox 布局",
            features: &["Row 容器", "水平排列", "样式填充"],
        },
        Example {
            name: "03_col_layout",
            description: "垂直布局 - 展示 Col 容器和 Flexbox 布局",
            features: &["Col 容器", "垂直排列", "样式填充"],
        },
        Example {
            name: "04_nested_containers",
            description: "嵌套容器 - 3层嵌套结构展示递归布局",
            features: &["多层嵌套", "混合布局", "递归计算"],
        },
        Example {
            name: "05_basic_edges",
            description: "基础连线 - 点对点连接展示",
            features: &["端口定义", "正交布线", "边渲染"],
        },
        Example {
            name: "06_complex_flow",
            description: "复杂流程 - 多步骤业务流",
            features: &["决策节点", "多端口", "流程分支", "合并点"],
        },
        Example {
            name: "07_high_density",
            description: "高密度图 - 拥塞反馈演示",
            features: &["20个节点", "密集连接", "拥塞检测", "空间扩展"],
        },
        Example {
            name: "08_microservices",
            description: "微服务架构 - 真实场景案例",
            features: &["服务边界", "数据库", "缓存", "外部服务", "多种形状"],
        },
    ];

    let mut results = Vec::new();

    println!("=== Tecton Showcase Generator ===\n");

    for example in &examples {
        let dsl_path = format!("{}/{}.tecton", EXAMPLES_DIR, example.name);
        let output_path = format!("{}/{}.svg", OUTPUTS_DIR, example.name);

        print!("Processing {}... ", example.name);

        let dsl = match fs::read_to_string(&dsl_path) {
            Ok(content) => content,
            Err(e) => {
                println!("ERROR: Failed to read {}: {}", dsl_path, e);
                results.push((example.name, false, 0.0, Some(e.to_string())));
                continue;
            }
        };

        let start = Instant::now();

        // Use the Tecton DSL parser (not yet implemented as public API)
        // For now, we'll use a placeholder that generates a simple SVG
        let svg = match generate_showcase_svg(&dsl, example.name) {
            Ok(svg) => svg,
            Err(e) => {
                println!("ERROR: {}", e);
                results.push((example.name, false, 0.0, Some(e.to_string())));
                continue;
            }
        };

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        match fs::write(&output_path, svg) {
            Ok(_) => {
                println!("OK ({:.2}ms)", elapsed);
                results.push((example.name, true, elapsed, None));
            }
            Err(e) => {
                println!("ERROR: Failed to write {}: {}", output_path, e);
                results.push((example.name, false, 0.0, Some(e.to_string())));
            }
        }
    }

    // Generate HTML gallery
    generate_showcase_html(&examples, &results)?;

    // Print summary
    println!("\n=== Summary ===");
    let success_count = results.iter().filter(|r| r.1).count();
    let total_count = results.len();
    println!("Generated: {}/{} examples", success_count, total_count);

    let total_time: f64 = results.iter().map(|r| r.2).sum();
    println!("Total time: {:.2}ms", total_time);

    if success_count == total_count {
        println!("\n✓ All examples generated successfully!");
        println!("  Gallery: showcase/index.html");
        println!("  Outputs: showcase/outputs/");
    } else {
        println!("\n✗ Some examples failed:");
        for (name, success, _, error) in &results {
            if !success {
                println!(
                    "  - {}: {}",
                    name,
                    error.as_ref().unwrap_or(&"Unknown error".to_string())
                );
            }
        }
    }

    Ok(())
}

/// Generate SVG for showcase using the actual Tecton rendering engine
fn generate_showcase_svg(dsl: &str, _name: &str) -> Result<String, String> {
    use tecton_engine::Tecton;

    // Call the real Tecton rendering API
    Tecton::render_tecton_dsl(dsl).map_err(|e| format!("Render failed: {}", e))
}

/// Generate the showcase HTML gallery
fn generate_showcase_html(
    examples: &[Example],
    results: &[(
        /*name*/ &str,
        /*success*/ bool,
        /*time*/ f64,
        /*error*/ Option<String>,
    )],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut html = String::new();

    html.push_str(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Tecton Showcase - Example Gallery</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f5f5f5;
            line-height: 1.6;
            color: #333;
        }
        .header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 2rem;
            text-align: center;
        }
        .header h1 { font-size: 2.5rem; margin-bottom: 0.5rem; }
        .header p { opacity: 0.9; font-size: 1.1rem; }
        .container {
            max-width: 1400px;
            margin: 0 auto;
            padding: 2rem;
        }
        .stats {
            display: flex;
            gap: 2rem;
            justify-content: center;
            margin: 2rem 0;
            flex-wrap: wrap;
        }
        .stat {
            background: white;
            padding: 1.5rem 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            text-align: center;
        }
        .stat-value {
            font-size: 2rem;
            font-weight: bold;
            color: #667eea;
        }
        .stat-label {
            color: #666;
            font-size: 0.9rem;
        }
        .examples-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(600px, 1fr));
            gap: 2rem;
        }
        .example-card {
            background: white;
            border-radius: 12px;
            overflow: hidden;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .example-card:hover {
            transform: translateY(-4px);
            box-shadow: 0 8px 12px rgba(0,0,0,0.15);
        }
        .example-header {
            padding: 1.5rem;
            border-bottom: 1px solid #eee;
            background: #fafafa;
        }
        .example-number {
            display: inline-block;
            background: #667eea;
            color: white;
            width: 28px;
            height: 28px;
            border-radius: 50%;
            text-align: center;
            line-height: 28px;
            font-size: 0.9rem;
            margin-right: 0.5rem;
        }
        .example-title {
            font-size: 1.25rem;
            font-weight: 600;
            color: #333;
        }
        .example-description {
            color: #666;
            margin-top: 0.5rem;
        }
        .example-svg {
            background: #fff;
            padding: 1rem;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 300px;
            border-bottom: 1px solid #eee;
        }
        .example-svg svg {
            max-width: 100%;
            height: auto;
            border: 1px solid #e0e0e0;
            border-radius: 4px;
        }
        .example-footer {
            padding: 1rem 1.5rem;
            background: #fafafa;
        }
        .features {
            display: flex;
            flex-wrap: wrap;
            gap: 0.5rem;
        }
        .feature-tag {
            background: #e3f2fd;
            color: #1976d2;
            padding: 0.25rem 0.75rem;
            border-radius: 16px;
            font-size: 0.8rem;
        }
        .status {
            display: inline-flex;
            align-items: center;
            gap: 0.25rem;
            font-size: 0.85rem;
            margin-left: auto;
        }
        .status.success { color: #4caf50; }
        .status.error { color: #f44336; }
        .footer {
            text-align: center;
            padding: 3rem 2rem;
            color: #666;
        }
        .footer a {
            color: #667eea;
            text-decoration: none;
        }
        .footer a:hover {
            text-decoration: underline;
        }
        pre {
            background: #263238;
            color: #aed581;
            padding: 1rem;
            overflow-x: auto;
            font-size: 0.85rem;
            line-height: 1.5;
        }
        .dsl-source {
            max-height: 200px;
            overflow-y: auto;
        }
        .tabs {
            display: flex;
            border-bottom: 1px solid #eee;
        }
        .tab {
            padding: 0.75rem 1.5rem;
            background: none;
            border: none;
            cursor: pointer;
            font-size: 0.9rem;
            color: #666;
            transition: background 0.2s;
        }
        .tab:hover {
            background: #f5f5f5;
        }
        .tab.active {
            color: #667eea;
            border-bottom: 2px solid #667eea;
        }
        .tab-content {
            display: none;
        }
        .tab-content.active {
            display: block;
        }
    </style>
</head>
<body>
    <header class="header">
        <h1>Tecton Showcase</h1>
        <p>Structured Architecture Diagram Rendering Engine - Example Gallery</p>
    </header>

    <div class="container">
        <div class="stats">
            <div class="stat">
                <div class="stat-value">"#,
    );

    html.push_str(&examples.len().to_string());
    html.push_str(
        r#"</div>
                <div class="stat-label">Examples</div>
            </div>
            <div class="stat">
                <div class="stat-value">"#,
    );

    let success_count = results.iter().filter(|r| r.1).count();
    html.push_str(&success_count.to_string());
    html.push_str(
        r#"</div>
                <div class="stat-label">Generated</div>
            </div>
            <div class="stat">
                <div class="stat-value">"#,
    );

    let total_time: f64 = results.iter().map(|r| r.2).sum();
    html.push_str(&format!("{:.1}ms", total_time));
    html.push_str(
        r#"</div>
                <div class="stat-label">Total Time</div>
            </div>
        </div>

        <div class="examples-grid">
"#,
    );

    // Add each example
    for (i, example) in examples.iter().enumerate() {
        let result = results.get(i).map(|r| (r.1, r.2)).unwrap_or((false, 0.0));
        let status_class = if result.0 { "success" } else { "error" };
        let status_icon = if result.0 { "✓" } else { "✗" };
        let status_text = if result.0 {
            format!("{:.2}ms", result.1)
        } else {
            "Failed".to_string()
        };

        html.push_str(&format!(
            r#"            <div class="example-card">
                <div class="example-header">
                    <span class="example-number">{}</span>
                    <span class="example-title">{}</span>
                    <span class="status {}">{} {}</span>
                    <p class="example-description">{}</p>
                </div>
                <div class="tabs">
                    <button class="tab active" onclick="showTab('{}', 'svg')">SVG Output</button>
                    <button class="tab" onclick="showTab('{}', 'dsl')">DSL Source</button>
                </div>
                <div id="{}-svg" class="tab-content active">
                    <div class="example-svg">
"#,
            i + 1,
            example.name.replace('_', " ").to_title_case(),
            status_class,
            status_icon,
            status_text,
            example.description,
            example.name,
            example.name,
            example.name
        ));

        // Include SVG inline if it exists
        let svg_path = format!("{}/{}.svg", OUTPUTS_DIR, example.name);
        if Path::new(&svg_path).exists() {
            if let Ok(svg_content) = fs::read_to_string(&svg_path) {
                // Extract just the svg element
                if let Some(start) = svg_content.find("<svg") {
                    if let Some(end) = svg_content.rfind("</svg>") {
                        html.push_str(&svg_content[start..end + 6]);
                    }
                }
            } else {
                html.push_str("<p>Failed to load SVG</p>");
            }
        } else {
            html.push_str("<p>SVG not generated</p>");
        }

        html.push_str(&format!(
            r#"                    </div>
                </div>
                <div id="{}-dsl" class="tab-content">
                    <pre class="dsl-source"><code>"#,
            example.name
        ));

        // Include DSL source
        let dsl_path = format!("{}/{}.tecton", EXAMPLES_DIR, example.name);
        if let Ok(dsl_content) = fs::read_to_string(&dsl_path) {
            html.push_str(&html_escape(&dsl_content));
        } else {
            html.push_str("Failed to load DSL");
        }

        html.push_str(&format!(
            r#"
</code></pre>
                </div>
                <div class="example-footer">
                    <div class="features">
"#
        ));

        for feature in example.features {
            html.push_str(&format!(
                r#"                        <span class="feature-tag">{}</span>
"#,
                feature
            ));
        }

        html.push_str(
            r#"                    </div>
                </div>
            </div>
"#,
        );
    }

    html.push_str(r#"        </div>
    </div>

    <footer class="footer">
        <p>Generated by <a href="https://github.com/tecton/tecton">Tecton</a> - Structured Architecture Diagram Rendering Engine</p>
        <p style="margin-top: 1rem; font-size: 0.85rem; color: #999;">
            Run <code>cargo run --bin showcase</code> to regenerate
        </p>
    </footer>

    <script>
        function showTab(exampleName, tabName) {
            // Hide all tab contents for this example
            const contents = document.querySelectorAll('[id^="' + exampleName + '-"]');
            contents.forEach(c => c.classList.remove('active'));

            // Show selected tab content
            document.getElementById(exampleName + '-' + tabName).classList.add('active');

            // Update tab buttons
            const tabs = document.querySelectorAll('[onclick*="' + exampleName + '"]');
            tabs.forEach(t => t.classList.remove('active'));
            event.target.classList.add('active');
        }
    </script>
</body>
</html>
"#);

    fs::write("showcase/index.html", html)?;
    println!("\n✓ Generated showcase/index.html");

    Ok(())
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Convert string to Title Case
trait TitleCase {
    fn to_title_case(&self) -> String;
}

impl TitleCase for str {
    fn to_title_case(&self) -> String {
        self.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
