#!/bin/bash
# AIW 快速测试脚本

AIW="$HOME/.local/bin/aiw"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

PASS=0
FAIL=0

echo "=== AIW v0.5.76 测试 ==="

# 1. 版本
if $AIW --version | grep -q "0.5.76"; then
    echo -e "${GREEN}✓${NC} 版本正确"
    ((PASS++))
else
    echo -e "${RED}✗${NC} 版本错误"
    ((FAIL++))
fi

# 2. ToolSearch 补丁
OUTPUT=$($AIW claude "test" 2>&1)
if echo "$OUTPUT" | grep -q "ToolSearch unlocked"; then
    echo -e "${GREEN}✓${NC} ToolSearch 补丁生效"
    ((PASS++))
else
    echo -e "${RED}✗${NC} ToolSearch 补丁失败"
    ((FAIL++))
fi

# 3. Worktree 创建
if echo "$OUTPUT" | grep -q "Created worktree"; then
    echo -e "${GREEN}✓${NC} Worktree 创建正常"
    ((PASS++))
else
    echo -e "${RED}✗${NC} Worktree 创建失败"
    ((FAIL++))
fi

# 4. 交互模式 AIW 退出
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

# 5. -C 参数
OUTPUT=$($AIW -C /tmp claude "test" 2>&1)
if echo "$OUTPUT" | grep -q "Starting"; then
    echo -e "${GREEN}✓${NC} -C 参数正常"
    ((PASS++))
else
    echo -e "${RED}✗${NC} -C 参数失败"
    ((FAIL++))
fi

# 6. CLI 参数转发
OUTPUT=$($AIW claude -m claude-3-7-sonnet "test" 2>&1)
if echo "$OUTPUT" | grep -q "Starting"; then
    echo -e "${GREEN}✓${NC} CLI 参数转发正常"
    ((PASS++))
else
    echo -e "${RED}✗${NC} CLI 参数转发失败"
    ((FAIL++))
fi

echo "通过: $PASS / 6"
[ $FAIL -eq 0 ] && exit 0 || exit 1
