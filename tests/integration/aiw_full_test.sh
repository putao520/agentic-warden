#!/bin/bash
# AIW 完整测试脚本 v0.5.61
#
# 测试覆盖:
# 1. CLI 类型 (Claude/Codex/Gemini) × 交互/非交互
# 2. 参数组合矩阵 (AIW 参数 + CLI 参数)
# 3. CLI 自有参数 (-m, -p, --max-tokens 等)
# 4. 交互/非交互判断逻辑
# 5. ToolSearch 补丁 (有/无 Provider, 交互/非交互模式)
#
# 使用安全模式避免阻塞: 后台运行 + sleep + kill，每个测试限制 5 秒

set -e

# ========================================
# 配置
# ========================================
AIW_BIN="${AIW_BIN:-cargo run --quiet --}"
AIW_INSTALL="${AIW_INSTALL:-$HOME/.local/bin/aiw}"
TEST_TIMEOUT=5
TEST_DIR="/tmp/aiw-test-$$"
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# 统计变量
PASS=0
FAIL=0
SKIP=0
TOTAL=0

# 创建测试目录
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"
git init -q
git config user.email "test@test.com"
git config user.name "Test"

# 清理函数
cleanup() {
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

# ========================================
# 测试辅助函数
# ========================================

# 运行测试并检查输出
run_test() {
    local desc="$1"
    local cmd="$2"
    local expect="$3"
    local should_fail="${4:-0}"

    ((TOTAL++))

    # 使用 timeout 避免阻塞
    local output
    output=$(timeout ${TEST_TIMEOUT}s bash -c "$cmd" 2>&1 || true)

    if [ $should_fail -eq 1 ]; then
        # 期望失败 (不应该包含 expect)
        if ! echo "$output" | grep -q "$expect"; then
            echo -e "${GREEN}[PASS]${NC} $desc"
            ((PASS++))
            return
        fi
    else
        # 期望成功 (应该包含 expect)
        if echo "$output" | grep -qiE "$expect"; then
            echo -e "${GREEN}[PASS]${NC} $desc"
            ((PASS++))
            return
        fi
    fi

    echo -e "${RED}[FAIL]${NC} $desc"
    echo -e "  Command: $cmd"
    echo -e "  Expected: $expect"
    echo -e "  Output: $(echo "$output" | head -c 200)"
    ((FAIL++))
}

# 运行交互模式测试 (后台 + kill)
run_interactive_test() {
    local desc="$1"
    local cmd="$2"
    local expect_behave="${3:-exit}"  # exit | stay

    ((TOTAL++))

    # 后台运行
    eval "$cmd" >/dev/null 2>&1 &
    local pid=$!
    sleep 2

    # 检查进程状态
    if ps -p $pid >/dev/null 2>&1; then
        # 进程还在运行
        if [ "$expect_behave" = "stay" ]; then
            echo -e "${GREEN}[PASS]${NC} $desc (进程保持运行)"
            ((PASS++))
        else
            echo -e "${RED}[FAIL]${NC} $desc (进程未退出，期望退出)"
            ((FAIL++))
        fi
        kill $pid 2>/dev/null || true
    else
        # 进程已退出
        if [ "$expect_behave" = "exit" ]; then
            echo -e "${GREEN}[PASS]${NC} $desc (进程已退出)"
            ((PASS++))
        else
            echo -e "${RED}[FAIL]${NC} $desc (进程退出，期望保持运行)"
            ((FAIL++))
        fi
    fi
}

# 检查 CLI 是否安装
cli_installed() {
    which "$1" >/dev/null 2>&1
}

# 打印章节
section() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}【$1】${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# 打印子章节
subsection() {
    echo ""
    echo -e "${CYAN}▶ $1${NC}"
}

# ========================================
# 预检查
# ========================================
section "预检查"

echo "测试目录: $TEST_DIR"
echo "AIW 命令: $AIW_BIN"

# 检查是否使用已安装的 aiw
if [[ -f "$AIW_BIN" ]] && [[ "$AIW_BIN" =~ aiw ]]; then
    echo "使用已安装的 aiw: $AIW_BIN"
else
    echo "使用 cargo run 构建 (可能较慢)"
fi

# 检测可用的 CLI
CLAUDE_INSTALLED=0
CODEX_INSTALLED=0
GEMINI_INSTALLED=0

if cli_installed claude; then
    echo -e "${GREEN}✓${NC} Claude CLI 已安装"
    CLAUDE_INSTALLED=1
else
    echo -e "${YELLOW}○${NC} Claude CLI 未安装 (跳过相关测试)"
fi

if cli_installed codex; then
    echo -e "${GREEN}✓${NC} Codex CLI 已安装"
    CODEX_INSTALLED=1
else
    echo -e "${YELLOW}○${NC} Codex CLI 未安装 (跳过相关测试)"
fi

if cli_installed gemini; then
    echo -e "${GREEN}✓${NC} Gemini CLI 已安装"
    GEMINI_INSTALLED=1
else
    echo -e "${YELLOW}○${NC} Gemini CLI 未安装 (跳过相关测试)"
fi

# ========================================
# 1. CLI 类型 × 交互/非交互
# ========================================
section "1. CLI 类型 × 交互/非交互模式"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    subsection "Claude CLI"

    run_interactive_test "Claude 无参数 → 交互模式" \
        "$AIW_BIN claude" \
        "exit"

    run_test "Claude 有 PROMPT → 非交互" \
        "$AIW_BIN claude 'test prompt'" \
        "worktree|Starting"

    run_test "Claude 空字符串 → 非交互" \
        "$AIW_BIN claude ''" \
        "worktree|Starting"
fi

if [ $CODEX_INSTALLED -eq 1 ]; then
    subsection "Codex CLI"

    run_interactive_test "Codex 无参数 → 交互模式" \
        "$AIW_BIN codex" \
        "exit"

    run_test "Codex 有 PROMPT → 非交互" \
        "$AIW_BIN codex 'test prompt'" \
        "worktree|Starting"
fi

if [ $GEMINI_INSTALLED -eq 1 ]; then
    subsection "Gemini CLI"

    run_interactive_test "Gemini 无参数 → 交互模式" \
        "$AIW_BIN gemini" \
        "exit"

    run_test "Gemini 有 PROMPT → 非交互" \
        "$AIW_BIN gemini 'test prompt'" \
        "worktree|Starting"
fi

# ========================================
# 2. 参数组合矩阵
# ========================================
section "2. AIW 参数组合"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    subsection "只有 AIW 参数"

    run_test "只有 -r role 参数" \
        "$AIW_BIN -r senior claude 'test'" \
        "worktree|Starting"

    run_test "只有 -C /tmp 参数" \
        "$AIW_BIN -C /tmp claude 'test'" \
        "worktree|Starting"

    run_test "AIW 参数组合: -r role -C /tmp" \
        "$AIW_BIN -r senior -C /tmp claude 'test'" \
        "worktree|Starting"

    subsection "只有 CLI 参数"

    run_test "只有 CLI -m 参数" \
        "$AIW_BIN claude -m claude-3-7-sonnet 'test'" \
        "worktree|Starting"

    run_test "只有 CLI --max-tokens 参数" \
        "$AIW_BIN claude --max-tokens 1000 'test'" \
        "worktree|Starting"

    subsection "AIW + CLI 混合参数"

    run_test "AIW -r + CLI -m" \
        "$AIW_BIN -r senior claude -m claude-3-7-sonnet 'test'" \
        "worktree|Starting"

    run_test "AIW -C + CLI -m" \
        "$AIW_BIN -C /tmp claude -m claude-3-7-sonnet 'test'" \
        "worktree|Starting"

    run_test "AIW -r -C + CLI -m" \
        "$AIW_BIN -r senior -C /tmp claude -m claude-3-7-sonnet 'test'" \
        "worktree|Starting"
fi

# ========================================
# 3. CLI 自有参数
# ========================================
section "3. CLI 自有参数处理"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    subsection "Claude 参数"

    run_test "Claude -m model 参数" \
        "$AIW_BIN claude -m claude-3-7-sonnet 'test'" \
        "worktree|Starting"

    run_test "Claude -p prompt 参数 (触发非交互)" \
        "$AIW_BIN claude -p 'test prompt'" \
        "worktree|Starting"

    run_test "Claude --max-tokens 参数" \
        "$AIW_BIN claude --max-tokens 1000 'test'" \
        "worktree|Starting"

    run_test "Claude -m + PROMPT 组合" \
        "$AIW_BIN claude -m claude-3-7-sonnet 'test prompt'" \
        "worktree|Starting"
fi

if [ $CODEX_INSTALLED -eq 1 ]; then
    subsection "Codex 参数"

    run_test "Codex -m model 参数" \
        "$AIW_BIN codex -m gpt-4 'test'" \
        "worktree|Starting"

    run_test "Codex --max-tokens 参数" \
        "$AIW_BIN codex --max-tokens 1000 'test'" \
        "worktree|Starting"
fi

if [ $GEMINI_INSTALLED -eq 1 ]; then
    subsection "Gemini 参数"

    run_test "Gemini -m model 参数" \
        "$AIW_BIN gemini -m gemini-2.0-flash 'test'" \
        "worktree|Starting"
fi

# ========================================
# 4. 交互/非交互判断
# ========================================
section "4. 交互/非交互判断逻辑"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    subsection "边界情况"

    run_interactive_test "无参数 → 交互" \
        "$AIW_BIN claude" \
        "exit"

    run_test "真实 PROMPT → 非交互" \
        "$AIW_BIN claude 'hello world'" \
        "worktree|Starting"

    run_test "只有 flags → 非交互 (flags 被视为 PROMPT)" \
        "$AIW_BIN claude --flag" \
        "worktree|Starting"

    run_test "空 PROMPT (空字符串) → 非交互" \
        "$AIW_BIN claude ''" \
        "worktree|Starting"

    run_test "多个空格后 PROMPT → 非交互" \
        "$AIW_BIN claude '   test   '" \
        "worktree|Starting"
fi

# ========================================
# 5. AIW -mp 参数
# ========================================
section "5. AIW Provider 参数 (-mp)"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    run_test "-mp provider 指定" \
        "$AIW_BIN -mp openrouter claude 'test'" \
        "worktree|Starting"

    run_test "-mp + -r 组合" \
        "$AIW_BIN -mp openrouter -r senior claude 'test'" \
        "worktree|Starting"

    run_test "--aiw-provider 长格式" \
        "$AIW_BIN --aiw-provider openrouter claude 'test'" \
        "worktree|Starting"
fi

# ========================================
# 6. ToolSearch 补丁
# ========================================
section "6. ToolSearch 补丁"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    subsection "非交互模式补丁"

    run_test "非交互模式 ToolSearch 补丁" \
        "$AIW_BIN claude 'test'" \
        "ToolSearch|unlocked"

    run_test "非交互模式 + Provider 补丁" \
        "$AIW_BIN -mp openrouter claude 'test'" \
        "ToolSearch|unlocked"

    subsection "交互模式补丁 (后台应用)"

    # 交互模式下补丁在后台应用，我们只检查进程正常启动
    run_interactive_test "交互模式后台补丁应用" \
        "$AIW_BIN claude" \
        "exit"
fi

# ========================================
# 7. Worktree 行为
# ========================================
section "7. Worktree 行为"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    run_test "非交互创建 worktree" \
        "$AIW_BIN claude 'test'" \
        "Created worktree"

    run_test "交互模式跳过 worktree (create_worktree=false)" \
        "$AIW_BIN claude" \
        "" \
        1  # 交互模式不应该有 worktree 输出

    run_test "-C 参数指定目录创建 worktree" \
        "$AIW_BIN -C /tmp claude 'test'" \
        "worktree"
fi

# ========================================
# 8. 错误处理
# ========================================
section "8. 错误处理"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    run_test "无效目录报错" \
        "$AIW_BIN -C /nonexistent-12345-dir claude 'test'" \
        "does not exist|not found|error"

    run_test "Auto CLI 直接执行报错" \
        "$AIW_BIN auto 'test'" \
        "only supported via|error" \
        1  # 应该报错
fi

# ========================================
# 9. 基本命令
# ========================================
section "9. 基本命令"

run_test "--version 输出" \
    "$AIW_BIN --version" \
    "aiw|0\.5"

run_test "--help 输出" \
    "$AIW_BIN --help" \
    "Usage|usage"

run_test "status 命令" \
    "$AIW_BIN status" \
    "CLI|Status|version"

# ========================================
# 10. 特殊场景
# ========================================
section "10. 特殊场景"

if [ $CLAUDE_INSTALLED -eq 1 ]; then
    # 测试多空格 prompt 处理
    run_test "Prompt 前后空格处理" \
        "$AIW_BIN claude '  test  '" \
        "worktree|Starting"

    # 测试特殊字符 prompt
    run_test "特殊字符 Prompt" \
        "$AIW_BIN claude 'test with \$PECIAL chars'" \
        "worktree|Starting"

    # 测试长 prompt
    run_test "长 Prompt 处理" \
        "$AIW_BIN claude 'this is a very long prompt with many words to test the argument handling'" \
        "worktree|Starting"
fi

# ========================================
# 总结
# ========================================
section "测试总结"

echo ""
echo -e "总计:   $TOTAL"
echo -e "${GREEN}通过:   $PASS${NC}"
if [ $FAIL -gt 0 ]; then
    echo -e "${RED}失败:   $FAIL${NC}"
else
    echo -e "失败:   $FAIL"
fi
echo -e "跳过:   $SKIP"
echo ""

SUCCESS_RATE=$((PASS * 100 / TOTAL))
echo -e "成功率: $SUCCESS_RATE%"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    echo -e "${GREEN}========================================${NC}"
    exit 0
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}✗ 有 $FAIL 个测试失败${NC}"
    echo -e "${RED}========================================${NC}"
    exit 1
fi
