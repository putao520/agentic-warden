#!/bin/bash
# AIW 交互模式自动化测试

AIW="$HOME/.local/bin/aiw"
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== AIW 交互模式测试 ==="
echo ""

# 方法1: 检查 AIW 是否被替换（通过检查 AIW 进程是否消失）
echo "方法1: 进程替换检查"
echo "启动 AIW..."
$AIW claude </dev/null >/dev/null 2>&1 &
AIW_PID=$!
sleep 2

if ps -p $AIW_PID 2>/dev/null; then
    echo -e "${RED}✗${NC} AIW 还在运行 (PID: $AIW_PID) - exec 失败"
    kill $AIW_PID 2>/dev/null
else
    echo -e "${GREEN}✓${NC} AIW 已被替换 (exec 成功)"
fi
echo ""

# 方法2: 使用 setsid 启动新会话
echo "方法2: setsid + timeout 检查"
timeout 3s setsid sh -c "$AIW claude 2>&1" 2>&1 | head -10 &
sleep 2
pgrep -claude | grep -v aiw | tail -2
echo ""

# 方法3: 检查新 Claude 进程的父进程
echo "方法3: 检查进程父进程 (PPID)"
$AIW claude 2>&1 &
PID=$!
sleep 2
# 查找新启动的 claude 进程（PPID 不是当前 shell）
NEW_CLAUDE=$(pgrep -claude | grep -v aiw | head -1)
if [ -n "$NEW_CLAUDE" ]; then
    PPID=$(ps -o ppid -p $NEW_CLAUDE 2>/dev/null)
    echo "新 Claude PID: $NEW_CLAUDE, PPID: $PPID"
    # 如果 PPID 是 1，说明父进程已退出
    if [ "$PPID" = "1" ]; then
        echo -e "${GREEN}✓${NC} PPID 是 1 (父进程已退出)"
    fi
fi
kill $PID 2>/dev/null || true
echo ""

# 方法4: 使用 expect 检查输出
if command -v expect >/dev/null 2>&1; then
    echo "方法4: expect 自动化"
    cat > /tmp/test_expect.exp << 'EXP'
        set timeout 3
        spawn $AIW claude
        expect {
            "ToolSearch" { puts "✓ 补丁成功"; exp_continue }
            timeout { puts "✗ 超时" }
            eof { puts "✓ EOF (进程替换成功)" }
        }
EXP
    expect /tmp/test_expect.exp
    rm -f /tmp/test_expect.exp
else
    echo "expect 未安装，跳过方法4"
fi

echo ""
echo "=== 测试完成 ==="
