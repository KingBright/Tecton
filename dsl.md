1. Tecton DSL 设计概览核心特性显式布局容器：直接支持 Row (水平), Col (垂直), Stack (层叠) 等关键字，对应 Android 的 LinearLayout 和 FrameLayout。权重驱动：使用 * 或 flex 关键字简写权重（类似 Android 的 layout_weight）。引用与连线分离：结构定义在代码块中，连线可以在任意地方引用 ID 进行连接。强类型形状：支持 Box, Circle, Diamond 等形状声明。2. DSL 语法示例让我们通过一个具体的例子（一个简单的微服务架构图）来展示这套 DSL：代码段// 1. 全局配置 (可选)
config {
    direction: LeftToRight
    gutter: 20px  // 默认间距
}

// 2. 样式定义 (类似 CSS 类)
class db_style {
    fill: #e1f5fe
    stroke: #0288d1
    shape: Cylinder // 数据库形状
}

// 3. 结构定义 (核心布局)
// 这是一个垂直的大容器 (Android LinearLayout vertical)
Col(padding: 20px, gap: 40px) {

    // 第一层：用户区
    // 居中放置
    Row(align: Center) {
        client
        mobile
    }

    // 第二层：API 网关
    // 使用权重，占据中间 100% 宽度
    Row(width: 100%) {
        gateway
    }

    // 第三层：后端服务 (水平排列)
    Row(gap: 30px) {
        // 左边服务群：占用 2 份空间
        Col(weight: 2, style: "border: 1px dashed #ccc") {
            label "Core Services" // 容器标签
            
            auth_svc
            user_svc
        }

        // 右边服务群：占用 1 份空间
        Col(weight: 1) {
            label "Analytics"
            
            data_svc
            
            // 应用样式类
            redis_db [label: "Cache", class: db_style]
        }
    }
}

// 4. 连线定义 (Topology)
// 可以在结构外部定义，支持链式写法
client -> gateway -> auth_svc
mobile -> gateway

// 跨层级连线，引擎会自动路由
auth_svc -> redis_db : "读写"
user_svc -> redis_db
gateway -> data_svc : "异步日志"
3. DSL 关键语法细节A. 布局属性映射 (Android -> Tecton DSL)Android 概念Android XML 属性Tecton DSL 语法LinearLayout (H)orientation="horizontal"Row {... }LinearLayout (V)orientation="vertical"Col {... }Paddingandroid:padding="16dp"padding: 16Marginandroid:layout_margin="10dp"margin: 10Weightandroid:layout_weight="1"weight: 1 或简写为 * (语法糖)Gravityandroid:gravity="center"align: CenterMatch Parentlayout_width="match_parent"width: 100%Wrap Contentlayout_width="wrap_content"width: Auto (默认)B. 语法糖：快速权重 (Sugar Syntax)为了让“工整”的布局写起来更快，我们可以借鉴 CSS Grid 的 fr 单位概念，或者 Rust 宏的风格：代码段// 快速定义一个三列布局，中间宽，两边窄
Row {
    sidebar [weight: 1]
    content [weight: 3]
    ads     [weight: 1]
}
4. 技术实现方案在 Rust 中实现这套 DSL，建议不使用笨重的 XML/JSON，而是编写一个定制的解析器。推荐技术栈：Pest 或 Winnow我推荐使用 Pest，因为它使用 PEG（解析表达文法），非常适合定义这种嵌套结构，且错误提示对用户很友好。模块设计：tecton_dsl我们需要在之前的架构中增加一个 dsl 模块：Grammar Definition (tecton.pest):代码段// 基础原子
WHITESPACE = _{ " " | "\t" | "\r" | "\n" }
identifier = @{ ASCII_ALPHA ~ (ASCII_ALPHANUMERIC | "_")* }
number     = @{ ASCII_DIGIT+ }

// 属性键值对: width: 100%
property   = { identifier ~ ":" ~ (string | number | identifier) }
props      = { "(" ~ property ~ ("," ~ property)* ~ ")" }

// 节点定义: node_id [ key: val ]
node_stmt  = { identifier ~ ("[" ~ props ~ "]")? }

// 容器定义: Row(props) {... }
container_kind = { "Row" | "Col" | "Stack" }
container_stmt = { container_kind ~ props? ~ "{" ~ (stmt)* ~ "}" }

// 连线: a -> b -> c
edge_op    = { "->" | "--" }
edge_stmt  = { identifier ~ (edge_op ~ identifier)+ ~ (":" ~ string)? }

// 根
stmt       = { container_stmt | node_stmt | edge_stmt }
file       = { SOI ~ stmt* ~ EOI }
AST 转换 (Parser Logic):解析器读取 .tcn 文件，生成一个中间的 AST (Abstract Syntax Tree)。关键步骤：在遍历 AST 时，遇到 Row/Col 容器，直接映射为 tecton_layout 中的 ContainerNode，并将 weight 等属性转换为 Taffy 引擎所需的 Style 结构体。宏展开 (Macro Expansion) (高级功能):未来可以支持类似 Repeat(4) { Node } 这样的语法，自动生成 4 个节点，方便画大规模集群图。5. 为什么这个 DSL 适合你的项目？符合直觉：Row 和 Col 完美契合你要求的“先整体后局部”的思维模式。用户通过嵌套这些容器，自然而然地就定义了“方正”的骨架。解决“均匀”问题：通过 weight 属性，用户可以显式控制空间分配（例如 1:2:1），而不是把这种控制权完全交给难以预测的力导向算法。Rust 亲和力：这种结构化数据非常容易映射到 Rust 的 Enum 和 Structs 上，解析性能极高。
