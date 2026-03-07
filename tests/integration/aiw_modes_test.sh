#!/bin/bash
# AIW 快速测试脚本

AIW="$HOME/.local/bin/aiw"
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PASS=0
FAIL=0
SKIP=0

echo "========================================"
echo "AIW 测试矩阵"
echo "========================================"
echo ""

run_test() {
    local desc="$1"
    local cmd="$2"
    local expect="$3"
    
    # 使用 timeout 避免阻塞
    local output
    output=$(timeout 5s bash -c "$cmd" 2>&1 || true)
    
    if echo "$output" | grep -q "$expect"; then
        echo -e "${GREEN}✓${NC} $desc"
        ((PASS++))
    else
        echo -e "${RED}✗${NC} $desc"
        ((FAIL++))
    fi
}

section() {
    echo ""
    echo -e "${BLUE}【$1】${NC}"
}

# ========================================
# 1. 交互/非交互
# ========================================
section "1. 交互/非交互模式"

run_test "无参数 → 交互模式" \
    "$AIW claude" \
    "interactive mode"

run_test "有提示词 → 非交互" \
    "$AIW claude test" \
    "with task:"

# ========================================
# 2. ToolSearch 补丁
# ========================================
section "2. ToolSearch 补丁"

run_test "ToolSearch 补丁生效" \
    "$AIW claude test" \
    "ToolSearch unlocked"

# ========================================
# 3. Worktree
# ========================================
section "3. Worktree 行为"

run_test "非交互创建 worktree" \
    "$AIW claude test" \
    "Created worktree"

# ========================================
# 4. 参数传递
# ========================================
section "4. 参数传递"

run_test "-C 参数" \
    "$AIW -C /tmp claude test" \
    "Starting"

run_test "CLI -m 参数" \
    "$AIW claude -m claude-3-7-sonnet test" \
    "Starting"

run_test "CLI -p 参数" \
    "$AIW claude -p test" \
    "with task"

# ========================================
# 5. 基本命令
# ========================================
section "5. 基本命令"

run_test "--version" \
    "$AIW --version" \
    "aiw"

run_test "--help" \
    "$AIW --help" \
    "Usage"

# ========================================
# 6. 错误处理
# ========================================
section "6. 错误处理"

run_test "无效目录报错" \
    "$AIW -C /nonexistent claude test" \
    "error\|Error\|not exist"

# ========================================
# 7. 进程行为
# ========================================
section "7. 进程行为"

# 交互模式应该立即退出
$AIW claude 2>&1 &
PID=$!
sleep 1
if ps -p $PID >/dev/null 2>&1; then
    echo -e "${RED}✗${NC} 交互模式 AIW 未退出"
    ((FAIL++))
    kill $PID 2>/dev/null || true
else
    echo -e "${GREEN}✓${NC} 交互模式 AIW 正确退出"
    ((PASS++))
fi

# ========================================
# 总结
# ========================================
echo ""
echo "========================================"
echo "通过: $PASS | 失败: $FAIL | 跳过: $SKIP"
echo "总计: $((PASS + FAIL + SKIP))"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}✗ 有 $FAIL 个测试失败${NC}"
    exit 1
fi
