Tecton: 结构化架构图渲染引擎技术方案1. 系统架构概览Tecton 的核心理念是将UI 布局的确定性（基于约束）与VLSI 物理设计的智能性（基于拥塞和密度）相结合。系统是一个闭环的反馈控制系统，而非单向流水线。核心工作流 (The Tecton Pipeline)语义解析 (Parsing): 将 DSL 转换为逻辑树（DOM Tree）和拓扑图（Topology Graph）。宏观规划 (Floorplanning): 使用加权递归二分法将画布切割为若干“地块”，确保整体方正且密度均匀。微观布局 (Detailed Placement): 在每个地块内，使用 Taffy (Flexbox) 进行局部元素的精确排列。布线预演 (Global Routing): 在不生成具体路径的情况下，基于概率估算连线所需的空间（Routing Channel）。拥塞反馈 (Congestion Feedback): (关键步骤) 检测拥堵区域，强制增加相关容器的 Padding/Margin，触发重布局。精细布线 (Detailed Routing): 在无拥塞的最终布局上，运行正交寻路算法生成连线。渲染 (Rendering): 输出图形。2. 模块拆分与接口定义 (Crate-Level Design)为了确保模块的独立性，建议采用 Rust Workspace 结构管理以下 crates。🧱 模块一：tecton_types (基础类型层)职责：定义全系统通用的数据结构，不包含复杂逻辑。所有其他模块都依赖此模块。Structs:NodeId: 唯一标识符 (u64)。VisualMass: 节点的布局权重结构 (包含 content_area, connection_density)。Rect: 基础几何结构 (x, y, w, h)。Port: 连线锚点 (Top, Bottom, Left, Right, Absolute)。Style: 类似 CSS 的样式定义。Traits:Measurable: 任何需要计算尺寸的物体必须实现的接口。📐 模块二：tecton_layout (布局计算层)职责：负责“盒子”的摆放。它不知道具体的连线路径，只关心如何预留空间。依赖: tecton_types, taffy (第三方库).核心组件:MassCalculator: 遍历 DOM 树，计算每个容器的权重。权重公式：$W = W_{self} + \sum W_{children} + k \cdot N_{edges}$。BisectionFloorplanner: 实现递归二分算法（Recursive Bisection）。输入总权重和总面积，输出每个顶级容器的建议 TargetRect，确保长宽比接近 1:1。TaffyAdapter: 将 tecton_types 的约束转换为 Taffy 的 Style，执行 Flexbox 布局。Spacer: 接收来自 orchestrator 的“膨胀指令”，动态修改节点的 Padding。🛣️ 模块三：tecton_routing (智能布线层)职责：在已知障碍物的情况下寻找最佳路径。它不修改节点位置，只负责画线或报错。依赖: tecton_types, pathfinding (或自定义 A*).核心组件:GridGraph: 将连续坐标离散化为网格（Grid）。每个格点存储 Cost（空白=1，边缘=5，障碍=∞）。VirtualLayer: 借鉴 VLSI，引入“虚拟层”概念。如果平面布线无解，允许临时使用“桥接层”（代价极高），并在结果中标记为“拥塞”。OARSMT_Solver: 障碍物回避矩形斯坦纳树求解器。用于计算多点连接（Hyperedge）或点对点连接的最佳正交路径。CongestionScanner: 分析网格热度，返回 CongestionReport（包含拥堵区域的 Rect 和严重程度）。🎨 模块四：tecton_render (渲染层)职责：将布局和布线产生的纯数据转换为可视格式。依赖: tecton_types.核心组件:BackendTrait: 定义绘制接口 (draw_rect, draw_path)。SvgBackend: 实现 SVG 字符串生成。DebugBackend: 一个特殊的渲染器，用于在图形上叠加显示“网格热力图”和“包围盒”，方便调试算法。🧠 模块五：tecton_engine (编排与控制层)职责：上帝视角。持有完整的 DOM 和图结构，协调 Layout 和 Routing 的迭代。依赖: 所有上述模块, petgraph.核心组件:Orchestrator: 状态机，控制迭代循环。LayoutGraph: 使用 petgraph 存储节点间的拓扑关系。DSL Parser: 解析用户输入的 JSON/YAML。3. 核心算法细节与 Rust 实现策略3.1 密度均衡算法 (Density Equalization)为了满足“内容分布均匀，不拥挤不空洞”：算法选择: Weighted Recursive Bisection (WRB)。实现逻辑:将根容器视为一个矩形 $R$。计算所有子节点的总视觉质量 $TotalMass$。找到一个分割点，将子节点分为两组 $A$ 和 $B$，使得 $Mass(A) \approx Mass(B) \approx TotalMass / 2$。决定切割方向：如果 $R.width > R.height$，垂直切割。否则，水平切割。Rust 技巧: 使用 match 匹配长宽比枚举。递归执行，直到处理完所有容器。效果: 这保证了每个区域的面积与其“内容复杂度”成正比，且整体趋向于正方形网格，避免了细长条布局。3.2 拥塞驱动的反馈循环 (Congestion-Driven Loop)这是解决“连线重叠”和“空间预留”的核心。Rust// 伪代码演示 Orchestrator 的核心逻辑
fn layout_loop(&mut self) -> Result<(), Error> {
    let mut margin_map = HashMap::new();
    
    for iteration in 0..MAX_ITERATIONS {
        // 1. 布局阶段
        let layout_result = self.layout_engine.compute(&self.dom, &margin_map);
        
        // 2. 构建路由网格
        let grid = self.routing_engine.build_grid(&layout_result);
        
        // 3. 试探性布线与拥塞检测
        let congestion_report = self.routing_engine.detect_congestion(&grid, &self.connections);
        
        if congestion_report.is_empty() {
            // 成功：无严重拥塞，执行最终精细布线
            self.final_routes = self.routing_engine.route_all(&grid, &self.connections);
            return Ok(());
        }
        
        // 4. 失败：根据拥塞报告调整边距
        // 如果节点A和B之间拥堵，强制增加它们之间的 Padding
        for hotspot in congestion_report.hotspots {
            let spacing_increase = hotspot.severity * STEP_SIZE;
            *margin_map.entry(hotspot.node_id).or_insert(0.0) += spacing_increase;
        }
    }
    Err(Error::ConvergenceFailed)
}
3.3 正交连线 (Orthogonal Routing)网格化: 所有的坐标在布线层都必须对齐到 GridUnit (例如 10px)。A 代价函数设计*:Rustfn heuristic(a: Point, b: Point) -> u32 {
    let dist = (a.x - b.x).abs() + (a.y - b.y).abs();
    let bend_penalty = if current_dir!= prev_dir { 10 } else { 0 };
    dist + bend_penalty
}
增加 bend_penalty 会强迫算法优先画直线，直到不得不转弯，从而生成“工整”的图形。4. 任务拆分 (Task Breakdown)Phase 1: 基础骨架 (Weeks 1-2)[ ] Task 1.1: 创建 Rust Workspace 和 5 个 crate。[ ] Task 1.2: 在 tecton_types 中定义 Node, Edge, Style 结构体。[ ] Task 1.3: 在 tecton_render 中实现一个简单的 SVG 输出器（只画矩形）。[ ] Task 1.4: 在 tecton_layout 中集成 taffy，实现从 JSON 到 Flexbox 布局的转换。里程碑: 输入 JSON，能生成一个嵌套的矩形框 SVG，没有连线，位置由 Flexbox 决定。Phase 2: 宏观规划与权重 (Weeks 3-4)[ ] Task 2.1: 实现 MassCalculator，递归计算节点权重。[ ] Task 2.2: 实现 Recursive Bisection 算法，根据权重分配顶级容器的 width/height。[ ] Task 2.3: 将 Task 2.2 的结果作为约束传递给 taffy。里程碑: 输入包含大量数据的 JSON，生成的矩形框布局呈现出“方正”、“均匀”的特性，而不是简单的从上到下罗列。Phase 3: 路由引擎与网格 (Weeks 5-6)[ ] Task 3.1: 实现 GridGraph，将布局结果映射为 0/1 矩阵。[ ] Task 3.2: 实现基础的 A* 寻路，支持正交约束。[ ] Task 3.3: 实现“端口选择”逻辑，自动选择最近的节点边界点。里程碑: 在 Phase 2 的基础上，矩形之间出现了连线。连线会避开矩形内部，但在拥挤处可能会重叠。Phase 4: 反馈闭环与优化 (Weeks 7-8)[ ] Task 4.1: 实现 CongestionScanner，识别连线重叠和过密的区域。[ ] Task 4.2: 在 tecton_engine 中实现迭代循环，连接 Layout 和 Routing。[ ] Task 4.3: 调试“膨胀”逻辑，确保增加 Margin 后 Taffy 能正确推挤周围元素。里程碑: 连线不再重叠。当连线很多时，盒子会自动“弹开”为连线留出通道。Phase 5: 渲染打磨 (Week 9)[ ] Task 5.1: 优化连线拐角（圆角处理）。[ ] Task 5.2: 添加箭头、标签文本的渲染。[ ] Task 5.3: 最终的 API 封装和文档。5. 为什么这个方案能成功？Rust 的优势: taffy 提供了现成的、工业级的布局算法，避免了手写 Flexbox 的巨坑。Rust 的所有权机制能很好地管理复杂的图数据结构。解决痛点:方正: 通过 Recursive Bisection 强行约束长宽比。均匀: 通过 Visual Mass 权重计算，内容多的节点自动分得更多空间。无重叠: 通过 VLSI 拥塞反馈，物理上推开空间，这是纯图论算法（如 Graphviz）难以做到的。
