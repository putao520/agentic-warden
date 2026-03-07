#!/bin/bash
# AIW 快速测试 - 避免阻塞的关键版本

AIW="${AIW_BIN:-$HOME/.local/bin/aiw}"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

PASS=0
FAIL=0
TOTAL=0

echo "=== AIW v0.5.76 快速测试 ==="
echo "AIW: $AIW"
echo ""

# 安全运行：超时 + 后台 kill
safe_test() {
    local desc="$1"
    local cmd="$2"
    local expect="$3"
    
    ((TOTAL++))
    
    # 后台运行，2秒后清理
    OUTPUT=$(timeout 2s bash -c "$cmd" 2>&1 || true)
    
    if echo "$OUTPUT" | grep -qE "$expect"; then
        echo -e "${GREEN}✓${NC} [$TOTAL] $desc"
        ((PASS++))
    else
        echo -e "${RED}✗${NC} [$TOTAL] $desc (期望: $expect)"
        ((FAIL++))
    fi
}

# 1. 版本
safe_test "版本检查" "$AIW --version" "aiw.*0\.5\.76"

# 2. ToolSearch
safe_test "ToolSearch 补丁" "$AIW claude test" "ToolSearch"

# 3. Worktree
safe_test "Worktree 创建" "$AIW claude test" "Created worktree"

# 4. 交互模式退出
$AIW claude 2>&1 &
PID=$!
sleep 1
((TOTAL++))
if ps -p $PID >/dev/null 2>&1; then
    echo -e "${RED}✗${NC} [$TOTAL] 交互模式 AIW 退出"
    ((FAIL++))
    kill $PID 2>/dev/null || true
else
    echo -e "${GREEN}✓${NC} [$TOTAL] 交互模式 AIW 退出"
    ((PASS++))
fi

# 5. -C 参数
safe_test "-C 参数" "$AIW -C /tmp claude test" "Starting"

# 6. CLI -m 参数
safe_test "CLI -m 参数" "$AIW claude -m claude-3-7-sonnet test" "Starting"

# 7. CLI -p 参数
safe_test "CLI -p 参数" "$AIW claude -p test" "with task"

# 8. 参数组合
safe_test "AIW + CLI 组合" "$AIW -C /tmp claude -m claude-3-7-sonnet test" "Starting"

# 9. 空字符串 PROMPT
safe_test "空字符串 PROMPT" "$AIW claude ''" "with task"

# 10. 错误处理
OUTPUT=$($AIW -C /nonexistent claude test 2>&1 || true)
((TOTAL++))
if echo "$OUTPUT" | grep -qiE "error|not exist"; then
    echo -e "${GREEN}✓${NC} [$TOTAL] 错误处理"
    ((PASS++))
else
    echo -e "${RED}✗${NC} [$TOTAL] 错误处理"
    ((FAIL++))
fi

echo ""
echo "========================================"
echo "通过: $PASS / $TOTAL | 失败: $FAIL"
echo "========================================"

[ $FAIL -eq 0 ] && exit 0 || exit 1
