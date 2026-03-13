#!/bin/bash
# Tecton 全量测试脚本
# 运行所有测试并生成报告

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

TIMESTAMP=$(date "+%Y%m%d_%H%M%S")
REPORT_DIR="test_reports"
mkdir -p "$REPORT_DIR"

echo "=== Tecton 全量测试 ==="
echo "时间: $(date)"
echo ""

# 1. 单元测试
echo ">>> 单元测试"
cargo test --workspace --lib 2>&1 | tee "$REPORT_DIR/unit_tests_$TIMESTAMP.txt" || true

# 2. 集成测试
echo ""
echo ">>> 集成测试"
cargo test --test '*' 2>&1 | tee "$REPORT_DIR/integration_tests_$TIMESTAMP.txt" || true

# 3. 文档测试
echo ""
echo ">>> 文档测试"
cargo test --doc --workspace 2>&1 | tee "$REPORT_DIR/doc_tests_$TIMESTAMP.txt" || true

# 4. 代码检查
echo ""
echo ">>> 代码检查 (Clippy)"
cargo clippy --workspace 2>&1 | tee "$REPORT_DIR/clippy_$TIMESTAMP.txt" || true

# 5. 生成报告
echo ""
echo "=== 测试结果汇总 ==="
echo ""

# 计算测试总数
UNIT_PASSED=$(grep -c "test result: ok" "$REPORT_DIR/unit_tests_$TIMESTAMP.txt" 2>/dev/null || echo "0")
echo "单元测试结果: $UNIT_PASSED 个模块通过"

# 6. 生成 Markdown 报告
cat > "$REPORT_DIR/TEST_REPORT_$TIMESTAMP.md" << EOF
# Tecton 测试报告

生成时间: $(date)

## 测试环境
- 操作系统: $(uname -s) $(uname -r)
- Rust 版本: $(rustc --version 2>/dev/null || echo "Unknown")
- Cargo 版本: $(cargo --version 2>/dev/null || echo "Unknown")

## 测试统计

| 测试类型 | 状态 |
|----------|------|
| 单元测试 | 见详细报告 |
| 集成测试 | 见详细报告 |
| 文档测试 | 见详细报告 |
| Clippy   | 见详细报告 |

## 详细报告文件
- 单元测试: unit_tests_$TIMESTAMP.txt
- 集成测试: integration_tests_$TIMESTAMP.txt
- 文档测试: doc_tests_$TIMESTAMP.txt
- Clippy: clippy_$TIMESTAMP.txt

## 覆盖率目标
- 整体: 70%+
- 核心模块 (tecton_types, tecton_layout): 80%+

---
*由 scripts/run_all_tests.sh 自动生成*
EOF

echo ""
echo "测试报告已生成: $REPORT_DIR/TEST_REPORT_$TIMESTAMP.md"
echo ""
echo "=== 完成 ==="
