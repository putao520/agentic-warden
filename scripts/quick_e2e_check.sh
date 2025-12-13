#!/bin/bash

# Agentic-Warden 快速E2E检查
# 验证核心功能是否正常工作

set -e

echo "🚀 Agentic-Warden 快速E2E功能检查"
echo "================================"

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 计数器
TOTAL=0
PASSED=0

# 测试函数
test_step() {
    local name="$1"
    local command="$2"
    local expected="$3"

    echo -n "🧪 $name ... "
    ((TOTAL++))

    if eval "$command" 2>&1 | grep -q "$expected"; then
        echo -e "${GREEN}✅ 通过${NC}"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}❌ 失败${NC}"
        return 1
    fi
}

echo ""
echo "📦 构建检查"
test_step "项目构建" "cargo build --release" "Finished"

echo ""
echo "💻 CLI基础功能"
test_step "CLI帮助" "./target/release/aiw --help" "AI CLI manager"
test_step "CLI状态" "./target/release/aiw status" "No tasks"
test_step "CLI版本" "./target/release/aiw --version" "5.1.1"

echo ""
echo "⚙️  MCP配置管理"
# 备份现有配置
[ -f ~/.aiw/mcp.json ] && cp ~/.aiw/mcp.json ~/.aiw/mcp.json.backup

test_step "添加MCP服务器" \
    "echo '' | ./target/release/aiw mcp add test-echo echo 'Hello MCP'" \
    "Added MCP server"

test_step "列出MCP服务器" \
    "./target/release/aiw mcp list" \
    "test-echo"

test_step "禁用MCP服务器" \
    "echo '' | ./target/release/aiw mcp disable test-echo" \
    "Disabled"

test_step "启用MCP服务器" \
    "echo '' | ./target/release/aiw mcp enable test-echo" \
    "Enabled"

echo ""
echo "🔄 进程追踪功能"
test_step "wait命令" \
    "timeout 5s ./target/release/aiw wait --timeout 3s" \
    "任务执行完成报告"

test_step "pwait命令" \
    "./target/release/aiw pwait \$\$" \
    "No tasks found"

echo ""
echo "🛠️  错误处理"
test_step "无效命令" \
    "./target/release/aiw invalid-command 2>&1" \
    "Unrecognized"

test_step "不存在的MCP服务器" \
    "./target/release/aiw mcp get nonexistent 2>&1" \
    "not found"

echo ""
echo "🧹 清理"
# 恢复配置
[ -f ~/.aiw/mcp.json.backup ] && mv ~/.aiw/mcp.json.backup ~/.aiw/mcp.json || rm -f ~/.aiw/mcp.json

echo ""
echo "📊 检查结果"
echo "============"
echo "总测试数: $TOTAL"
echo "通过测试: $PASSED"
echo "失败测试: $((TOTAL - PASSED))"

if [ $PASSED -eq $TOTAL ]; then
    echo -e "${GREEN}🎉 所有E2E检查通过！${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  有 $((TOTAL - PASSED)) 个检查失败${NC}"
    exit 1
fi