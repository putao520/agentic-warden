#!/bin/bash

# Agentic-Warden 功能需求E2E测试脚本
# 覆盖CLI调用、任务追踪、MCP配置、RMCP生命周期等核心功能

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 配置
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/release"
TEST_RESULTS_DIR="$PROJECT_ROOT/test-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
REPORT_FILE="$TEST_RESULTS_DIR/e2e_report_$TIMESTAMP.md"

# 创建测试结果目录
mkdir -p "$TEST_RESULTS_DIR"

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$REPORT_FILE"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1" | tee -a "$REPORT_FILE"
    ((PASSED_TESTS++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1" | tee -a "$REPORT_FILE"
    ((FAILED_TESTS++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$REPORT_FILE"
}

log_test() {
    echo -e "${PURPLE}[TEST]${NC} $1" | tee -a "$REPORT_FILE"
    ((TOTAL_TESTS++))
}

# 测试函数
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_pattern="$3"
    local timeout="${4:-30}"

    log_test "$test_name"

    if timeout "$timeout" bash -c "$test_command" 2>&1 | tee -a "$REPORT_FILE" | grep -q "$expected_pattern"; then
        log_success "$test_name"
        return 0
    else
        log_error "$test_name"
        return 1
    fi
}

# 初始化报告
init_report() {
    cat > "$REPORT_FILE" << EOF
# Agentic-Warden E2E 功能测试报告

**测试时间**: $(date)
**项目根目录**: $PROJECT_ROOT
**构建目录**: $BUILD_DIR

## 测试概览

EOF
}

# 生成最终报告
finalize_report() {
    cat >> "$REPORT_FILE" << EOF

## 测试总结

- **总测试数**: $TOTAL_TESTS
- **通过测试**: $PASSED_TESTS
- **失败测试**: $FAILED_TESTS
- **成功率**: $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%

## 详细结果

EOF

    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}✅ 所有E2E测试通过！${NC}" | tee -a "$REPORT_FILE"
    else
        echo -e "${RED}❌ 有 $FAILED_TESTS 个测试失败${NC}" | tee -a "$REPORT_FILE"
    fi
}

# 测试1: CLI基础功能
test_cli_basic_functionality() {
    log_info "=== 测试1: CLI基础功能 ==="

    # 测试帮助命令
    run_test "CLI帮助命令" \
        "$BUILD_DIR/aiw --help" \
        "AI CLI manager with process tracking"

    # 测试状态命令
    run_test "CLI状态命令" \
        "$BUILD_DIR/aiw status" \
        "No tasks"

    # 测试版本信息
    run_test "CLI版本信息" \
        "$BUILD_DIR/aiw --version" \
        "5.1.1"
}

# 测试2: MCP配置管理
test_mcp_configuration() {
    log_info "=== 测试2: MCP配置管理 ==="

    # 清理现有配置
    rm -f ~/.aiw/mcp.json

    # 测试添加MCP服务器
    run_test "添加filesystem MCP服务器" \
        "echo '' | $BUILD_DIR/aiw mcp add filesystem npx @modelcontextprotocol/server-filesystem /tmp" \
        "Added MCP server"

    # 测试列出MCP服务器
    run_test "列出MCP服务器" \
        "$BUILD_DIR/aiw mcp list" \
        "filesystem.*enabled"

    # 测试禁用服务器
    run_test "禁用MCP服务器" \
        "echo '' | $BUILD_DIR/aiw mcp disable filesystem" \
        "Disabled MCP server"

    # 测试启用服务器
    run_test "启用MCP服务器" \
        "echo '' | $BUILD_DIR/aiw mcp enable filesystem" \
        "Enabled MCP server"
}

# 测试3: MCP服务器启动和RMCP路由
test_mcp_server_startup() {
    log_info "=== 测试3: MCP服务器启动和RMCP路由 ==="

    # 测试MCP服务器启动（简短测试）
    run_test "MCP服务器启动" \
        "echo '{}' | timeout 10s $BUILD_DIR/aiw mcp serve 2>&1 | head -20" \
        "MCP.*ready" \
        15

    # 测试配置文件存在性
    run_test "MCP配置文件存在" \
        "test -f ~/.aiw/mcp.json && echo 'Config file exists'" \
        "Config file exists"
}

# 测试4: 任务启动和管理
test_task_management() {
    log_info "=== 测试4: 任务启动和管理 ==="

    # 首先构建测试工具
    log_info "构建任务测试工具..."
    cargo build --bin test_launch --release 2>/dev/null || true

    # 检查是否有可用的AI CLI
    if command -v codex >/dev/null 2>&1; then
        export CODEX_BIN=$(which codex)

        # 测试任务启动（非阻塞测试）
        run_test "任务启动功能" \
            "timeout 5s $BUILD_DIR/test_launch 2>&1 | head -10" \
            "Task launched successfully" \
            10

        # 测试任务状态检查
        run_test "任务状态检查" \
            "$BUILD_DIR/aiw status" \
            "No tasks"
    else
        log_warning "跳过任务管理测试 - codex CLI不可用"
    fi
}

# 测试5: 共享内存和进程追踪
test_process_tracking() {
    log_info "=== 测试5: 进程追踪功能 ==="

    # 测试pwait命令（预期没有任务）
    run_test "pwait命令功能" \
        "$BUILD_DIR/aiw pwait $$" \
        "No tasks found"

    # 测试wait命令
    run_test "wait命令功能" \
        "timeout 5s $BUILD_DIR/aiw wait --timeout 3s" \
        "任务执行完成报告"
}

# 测试6: 配置文件热重载
test_config_hot_reload() {
    log_info "=== 测试6: 配置热重载 ==="

    # 备份原配置
    if [ -f ~/.aiw/mcp.json ]; then
        cp ~/.aiw/mcp.json ~/.aiw/mcp.json.backup
    fi

    # 修改配置文件
    cat > ~/.aiw/mcp.json << 'EOF'
{
  "mcpServers": {
    "test-server": {
      "command": "echo",
      "args": ["hello"],
      "enabled": true
    }
  }
}
EOF

    # 验证配置变更被识别
    run_test "配置文件变更识别" \
        "$BUILD_DIR/aiw mcp list" \
        "test-server.*enabled"

    # 恢复原配置
    if [ -f ~/.aiw/mcp.json.backup ]; then
        mv ~/.aiw/mcp.json.backup ~/.aiw/mcp.json
    else
        rm -f ~/.aiw/mcp.json
    fi
}

# 测试7: 错误处理和边界情况
test_error_handling() {
    log_info "=== 测试7: 错误处理和边界情况 ==="

    # 测试无效命令
    run_test "无效命令处理" \
        "$BUILD_DIR/aiw invalid-command 2>&1" \
        "Unrecognized subcommand"

    # 测试不存在的MCP服务器
    run_test "不存在的MCP服务器" \
        "$BUILD_DIR/aiw mcp get nonexistent-server 2>&1" \
        "not found"

    # 测试无效PID的pwait
    run_test "无效PID的pwait" \
        "$BUILD_DIR/aiw pwait 999999 2>&1" \
        "No tasks found"
}

# 清理函数
cleanup() {
    log_info "清理测试环境..."

    # 清理临时文件
    rm -f "$PROJECT_ROOT"/test_launch.rs
    rm -f "$PROJECT_ROOT"/src/bin/test_launch.rs
    rm -f "$PROJECT_ROOT"/src/bin/list_tasks.rs
    rm -f "$PROJECT_ROOT"/test_mcp_hot_reload.py

    # 恢复MCP配置
    if [ -f ~/.aiw/mcp.json.backup ]; then
        mv ~/.aiw/mcp.json.backup ~/.aiw/mcp.json
    fi
}

# 主函数
main() {
    echo -e "${CYAN}🚀 开始Agentic-Warden E2E功能测试${NC}"

    # 设置清理陷阱
    trap cleanup EXIT

    # 初始化报告
    init_report

    # 检查构建
    if [ ! -f "$BUILD_DIR/aiw" ]; then
        log_info "构建Agentic-Warden..."
        cargo build --release
    fi

    # 运行测试套件
    test_cli_basic_functionality
    test_mcp_configuration
    test_mcp_server_startup
    test_task_management
    test_process_tracking
    test_config_hot_reload
    test_error_handling

    # 生成最终报告
    finalize_report

    echo -e "\n${CYAN}📊 测试报告已生成: $REPORT_FILE${NC}"

    # 返回适当的退出码
    if [ $FAILED_TESTS -eq 0 ]; then
        exit 0
    else
        exit 1
    fi
}

# 运行主函数
main "$@"