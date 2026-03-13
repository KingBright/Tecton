1. 核心策略：包围盒与几何修正 (The Box Model & Geometric Correction)
虽然人类看到的是圆或菱形，但**布局引擎（Layout Engine）**看到的必须是矩形。我们通过三个阶段来处理形状差异：

阶段 A：布局时 (Placement) -> 视作矩形 (AABB)

所有的形状（圆、菱形、平行四边形）都用其轴对齐包围盒 (Axis-Aligned Bounding Box, AABB) 参与 Taffy 的计算。

例如： 一个半径为 R 的圆，其布局尺寸就是 2R x 2R 的正方形。

阶段 B：布线时 (Routing) -> 视作障碍物掩码 (Obstacle Mask)

为了防止连线“穿过”圆的角落（虽然圆在矩形内，但角是空的），我们需要在网格中标记精确的占用区域。

或者更简单的做法：仍然把它们当作矩形障碍物。这样做的好处是布局更“透气”，连线不会贴着圆弧走，而是保持正直的间距，这符合“方正”的审美原则。

阶段 C：连接时 (Port Calculation) -> 视作精确几何 (Exact Geometry)

这是最关键的一步。连线不能停在包围盒的边缘，必须刚好停在形状的轮廓线上。

2. 技术实现补充：Shape Trait 与 端口计算
我们需要在 模块一 (tecton_types) 和 模块三 (tecton_routing) 中引入形状感知的逻辑。

2.1 定义 Shape Trait (Rust 代码示例)
在 tecton_types 中，我们不再只存储 width/height，而是存储一个枚举或特征对象。

Rust
// tecton_types/src/shapes.rs

use crate::geometry::{Point, Rect};

/// 定义所有节点形状必须实现的行为
pub trait ShapeGeometry {
    /// 获取布局引擎所需的包围盒尺寸
    fn measure(&self) -> (f32, f32);

    /// 给定连线方向，计算连线应该停在哪个坐标点 (Port)
    /// center: 节点的中心坐标
    /// incoming_dir: 连线进来的方向 (用于光线投射算法)
    fn get_port_position(&self, center: Point, incoming_dir: Direction) -> Point;
    
    /// (可选) 用于更精细的网格避障
    fn contains_point(&self, local_point: Point) -> bool;
}

// 具体实现：菱形 (常用于流程图判断节点)
pub struct DiamondShape {
    size: f32, 
}

impl ShapeGeometry for DiamondShape {
    fn measure(&self) -> (f32, f32) {
        // 菱形的包围盒是一个正方形
        (self.size, self.size)
    }

    fn get_port_position(&self, center: Point, incoming_dir: Direction) -> Point {
        // 菱形的连接点通常只允许在四个顶点上 (Top, Bottom, Left, Right)
        match incoming_dir {
            Direction::Top => Point::new(center.x, center.y - self.size / 2.0),
            Direction::Bottom => Point::new(center.x, center.y + self.size / 2.0),
            Direction::Left => Point::new(center.x - self.size / 2.0, center.y),
            Direction::Right => Point::new(center.x + self.size / 2.0, center.y),
        }
    }
}

// 具体实现：圆形
pub struct CircleShape {
    radius: f32,
}

impl ShapeGeometry for CircleShape {
    fn get_port_position(&self, center: Point, incoming_dir: Direction) -> Point {
        // 圆形的连接点是圆周上的任意一点，通常由连线角度决定
        // 简单的正交布局中，通常也是切到圆的 T/B/L/R 四个极点
        //... 计算极点坐标...
    }
}
2.2 路由引擎的适配
在 模块三 (tecton_routing) 中，我们需要修改 Port Solver 任务：

原逻辑：连线终点 = 矩形边缘。

新逻辑：

布线算法（A*）先寻路到矩形包围盒的中心或边缘网格点。

在生成最终路径坐标时，调用 node.shape.get_port_position() 进行**“末端修剪 (End-point Clipping)”**。

这样，连线会看起来完美地连接在圆弧或菱形尖端上，而不会悬空或刺入内部。

3. 更新后的任务清单 (针对异形支持)
我们需要在原计划的 Phase 1 和 Phase 3 中增加以下具体任务：

Phase 1: 基础骨架 (更新)
[ ] Task 1.2.1: 在 tecton_types 中定义 ShapeType 枚举 (Rectangle, Circle, Diamond, Cylinder, Parallelogram)。

[ ] Task 1.5: 实现 ShapeGeometry Trait，为每种形状编写数学公式（计算边缘交点）。

Phase 3: 路由引擎 (更新)
[ ] Task 3.3.1: 实现端口计算器 (Port Solver)。

输入：两个节点的中心坐标、形状类型。

输出：修正后的起点和终点坐标。

难点处理：对于菱形，如果不使用正交连线，线可能会切过菱形的空白区域。强制要求正交连线（Orthogonal） 可以完美规避这个问题，因为正交线天然垂直进入菱形的四个顶点。

Phase 5: 渲染 (更新)
[ ] Task 5.4: SVG 渲染器支持异形绘制。

<rect> 用于方框。

<circle> 或 <ellipse> 用于圆。

<path> (M x y L x y... z) 用于菱形和平行四边形。
