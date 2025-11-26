#!/bin/bash

# Agentic-Warden 测试运行器
# 提供统一的测试运行接口和报告生成

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 默认配置
DEFAULT_TEST_TYPE="all"
DEFAULT_OUTPUT_DIR="test-results"
DEFAULT_PARALLEL_JOBS=$(nproc)
DEFAULT_TIMEOUT_SECONDS=300

# 显示帮助信息
show_help() {
    cat << EOF
Agentic-Warden 测试运行器

用法: $0 [选项] [测试类型]

测试类型:
  unit                    - 运行单元测试
  integration            - 运行集成测试
  cli                    - 运行CLI测试
  tui                    - 运行TUI测试
  performance            - 运行性能测试
  coverage               - 生成代码覆盖率报告
  all                    - 运行所有测试 (默认)
  quick                  - 运行快速测试套件
  smoke                  - 运行冒烟测试

选项:
  -j, --jobs N           - 并行作业数 (默认: $DEFAULT_PARALLEL_JOBS)
  -t, --timeout N        - 测试超时时间，秒 (默认: $DEFAULT_TIMEOUT_SECONDS)
  -o, --output DIR       - 输出目录 (默认: $DEFAULT_OUTPUT_DIR)
  -v, --verbose          - 详细输出
  -q, --quiet            - 静默模式
  --no-fail-fast         - 遇到失败时继续运行其他测试
  --no-cache             - 不使用缓存
  --docker               - 在Docker容器中运行测试
  --help                 - 显示此帮助信息

环境变量:
  AGENTIC_WARDEN_TEST_MODE=1    - 启用测试模式
  SKIP_NETWORK_CALLS=1          - 跳过网络调用
  RUST_LOG=debug               - 设置日志级别
  RUST_BACKTRACE=1             - 显示详细错误信息

示例:
  $0                          # 运行所有测试
  $0 unit                     # 只运行单元测试
  $0 -j 8 integration         # 并行运行集成测试
  $0 -o /tmp/results coverage  # 生成覆盖率报告
  $0 --docker all              # 在Docker中运行所有测试

EOF
}

# 解析命令行参数
parse_args() {
    TEST_TYPE="$DEFAULT_TEST_TYPE"
    OUTPUT_DIR="$DEFAULT_OUTPUT_DIR"
    PARALLEL_JOBS="$DEFAULT_PARALLEL_JOBS"
    TIMEOUT="$DEFAULT_TIMEOUT_SECONDS"
    VERBOSE=false
    QUIET=false
    FAIL_FAST=true
    USE_CACHE=true
    DOCKER_MODE=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -j|--jobs)
                PARALLEL_JOBS="$2"
                shift 2
                ;;
            -t|--timeout)
                TIMEOUT="$2"
                shift 2
                ;;
            -o|--output)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -q|--quiet)
                QUIET=true
                shift
                ;;
            --no-fail-fast)
                FAIL_FAST=false
                shift
                ;;
            --no-cache)
                USE_CACHE=false
                shift
                ;;
            --docker)
                DOCKER_MODE=true
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            -*)
                echo "错误: 未知选项 $1"
                show_help
                exit 1
                ;;
            *)
                TEST_TYPE="$1"
                shift
                ;;
        esac
    done
}

# 日志函数
log_info() {
    if [ "$QUIET" != true ]; then
        echo -e "${BLUE}[INFO]${NC} $1"
    fi
}

log_success() {
    if [ "$QUIET" != true ]; then
        echo -e "${GREEN}[SUCCESS]${NC} $1"
    fi
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# 设置测试环境
setup_test_env() {
    log_info "设置测试环境..."

    # 创建输出目录
    mkdir -p "$OUTPUT_DIR"

    # 设置环境变量
    export AGENTIC_WARDEN_TEST_MODE=1
    export SKIP_NETWORK_CALLS=1
    export RUST_LOG=debug
    export RUST_BACKTRACE=1

    # 设置cargo缓存
    if [ "$USE_CACHE" = false ]; then
        export CARGO_TARGET_DIR="$OUTPUT_DIR/target"
    fi

    log_success "测试环境设置完成"
}

# 清理测试环境
cleanup_test_env() {
    log_info "清理测试环境..."

    # 清理临时文件
    if [ -d "/tmp/agentic-warden-test" ]; then
        rm -rf /tmp/agentic-warden-test
    fi

    log_success "测试环境清理完成"
}

# 运行单元测试
run_unit_tests() {
    log_info "运行单元测试..."

    local test_args="--lib"

    if [ "$VERBOSE" = true ]; then
        test_args="$test_args -- --nocapture"
    fi

    if [ "$FAIL_FAST" = false ]; then
        test_args="$test_args -- --no-fail-fast"
    fi

    if ! timeout "$TIMEOUT" cargo test $test_args 2>&1 | tee "$OUTPUT_DIR/unit_tests.log"; then
        log_error "单元测试失败"
        return 1
    fi

    log_success "单元测试通过"
    return 0
}

# 运行集成测试
run_integration_tests() {
    log_info "运行集成测试..."

    # SPEC CI容器化铁律警告
    if [ "$DOCKER_MODE" != true ] && [ "$CICD_MODE" != true ]; then
        log_warning "======================================================================"
        log_warning "⚠️  SPEC CI容器化测试铁律警告"
        log_warning ""
        log_warning "禁止在主机直接运行集成测试！"
        log_warning "所有非单元测试必须使用CI容器环境执行。"
        log_warning ""
        log_warning "正确方式："
        log_warning "  1. 使用Docker模式: $0 --docker integration"
        log_warning "  2. 使用CI/CD: 通过GitHub Actions或docker-compose.ci.yml"
        log_warning "  3. 启动CI环境: docker-compose -f docker-compose.ci.yml up"
        log_warning ""
        log_warning "立即终止集成测试执行。"
        log_warning "======================================================================"
        return 1
    fi

    local test_args="--test integration"

    if [ "$VERBOSE" = true ]; then
        test_args="$test_args -- --nocapture"
    fi

    if [ "$FAIL_FAST" = false ]; then
        test_args="$test_args -- --no-fail-fast"
    fi

    if ! timeout "$TIMEOUT" cargo test $test_args 2>&1 | tee "$OUTPUT_DIR/integration_tests.log"; then
        log_error "集成测试失败"
        return 1
    fi

    log_success "集成测试通过"
    return 0
}

# 运行CLI测试
run_cli_tests() {
    log_info "运行CLI测试..."

    # SPEC CI容器化铁律警告
    if [ "$DOCKER_MODE" != true ] && [ "$CICD_MODE" != true ]; then
        log_warning "======================================================================"
        log_warning "⚠️  SPEC CI容器化测试铁律警告"
        log_warning ""
        log_warning "禁止在主机直接运行CLI集成测试！"
        log_warning "所有非单元测试必须使用CI容器环境执行。"
        log_warning ""
        log_warning "正确方式："
        log_warning "  1. 使用Docker模式: $0 --docker cli"
        log_warning "  2. 使用CI/CD: 通过GitHub Actions"
        log_warning ""
        log_warning "立即终止CLI测试执行。"
        log_warning "======================================================================"
        return 1
    fi

    local test_args="--test cli_integration"

    if [ "$VERBOSE" = true ]; then
        test_args="$test_args -- --nocapture"
    fi

    if ! timeout "$TIMEOUT" cargo test $test_args 2>&1 | tee "$OUTPUT_DIR/cli_tests.log"; then
        log_error "CLI测试失败"
        return 1
    fi

    log_success "CLI测试通过"
    return 0
}

# 运行TUI测试
run_tui_tests() {
    log_info "运行TUI测试..."

    local test_args="--test tui_integration"

    if [ "$VERBOSE" = true ]; then
        test_args="$test_args -- --nocapture"
    fi

    # TUI测试需要特殊的环境设置
    export TERM=xterm-256color

    if ! timeout "$TIMEOUT" cargo test $test_args 2>&1 | tee "$OUTPUT_DIR/tui_tests.log"; then
        log_error "TUI测试失败"
        return 1
    fi

    log_success "TUI测试通过"
    return 0
}

# 运行性能测试
run_performance_tests() {
    log_info "运行性能测试..."

    if ! cargo criterion 2>&1 | tee "$OUTPUT_DIR/performance_tests.log"; then
        log_warning "性能测试失败或未运行"
        return 0 # 性能测试失败不应该阻止CI
    fi

    log_success "性能测试完成"
    return 0
}

# 生成代码覆盖率报告
generate_coverage() {
    log_info "生成代码覆盖率报告..."

    # 检查是否安装了cargo-llvm-cov
    if ! command -v cargo-llvm-cov &> /dev/null; then
        log_warning "cargo-llvm-cov未安装，跳过覆盖率报告"
        return 0
    fi

    # 生成覆盖率报告
    if ! cargo llvm-cov --workspace --lcov --output-path "$OUTPUT_DIR/lcov.info" \
        --html --output-dir "$OUTPUT_DIR/coverage" 2>&1 | tee "$OUTPUT_DIR/coverage.log"; then
        log_error "覆盖率报告生成失败"
        return 1
    fi

    log_success "覆盖率报告生成完成: $OUTPUT_DIR/coverage"
    return 0
}

# 运行快速测试
run_quick_tests() {
    log_info "运行快速测试套件..."

    # 只运行最重要的测试
    local test_args="--lib --test cli_integration -- --skip slow"

    if [ "$VERBOSE" = true ]; then
        test_args="$test_args -- --nocapture"
    fi

    if ! timeout "$TIMEOUT" cargo test $test_args 2>&1 | tee "$OUTPUT_DIR/quick_tests.log"; then
        log_error "快速测试失败"
        return 1
    fi

    log_success "快速测试通过"
    return 0
}

# 运行冒烟测试
run_smoke_tests() {
    log_info "运行冒烟测试..."

    # 构建项目
    if ! cargo build --release 2>&1 | tee "$OUTPUT_DIR/build.log"; then
        log_error "项目构建失败"
        return 1
    fi

    # 测试基本CLI功能
    if ! ./target/release/agentic-warden --version > /dev/null 2>&1; then
        log_error "版本命令失败"
        return 1
    fi

    if ! ./target/release/agentic-warden --help > /dev/null 2>&1; then
        log_error "帮助命令失败"
        return 1
    fi

    # 运行最关键的单元测试
    if ! timeout "$TIMEOUT" cargo test --lib test_provider_manager_creation 2>&1 | tee "$OUTPUT_DIR/smoke_tests.log"; then
        log_error "冒烟测试失败"
        return 1
    fi

    log_success "冒烟测试通过"
    return 0
}

# 在Docker中运行测试
run_in_docker() {
    log_info "在Docker容器中运行测试..."

    # 检查Docker是否可用
    if ! command -v docker &> /dev/null; then
        log_error "Docker不可用"
        return 1
    fi

    # 构建Docker镜像（如果不存在）
    if ! docker images agentic-warden-test &> /dev/null; then
        log_info "构建Docker测试镜像..."
        docker build -t agentic-warden-test -f Dockerfile.test . || {
            log_error "Docker镜像构建失败"
            return 1
        }
    fi

    # 运行Docker容器
    local docker_args="run --rm -v $(pwd):/workspace -w /workspace"

    if [ "$VERBOSE" = true ]; then
        docker_args="$docker_args -e RUST_LOG=debug"
    fi

    docker $docker_args agentic-warden-test /workspace/scripts/test_runner.sh "$@" || {
        log_error "Docker测试失败"
        return 1
    }

    log_success "Docker测试完成"
    return 0
}

# 生成测试报告
generate_report() {
    log_info "生成测试报告..."

    local report_file="$OUTPUT_DIR/test_report.md"

    cat > "$report_file" << EOF
# Agentic-Warden 测试报告

## 测试配置
- 测试类型: $TEST_TYPE
- 并行作业数: $PARALLEL_JOBS
- 超时时间: ${TIMEOUT}s
- 运行时间: $(date)
- Git提交: $(git rev-parse --short HEAD 2>/dev/null || echo "未知")

## 测试结果
EOF

    # 添加各测试结果
    for test_log in unit_tests.log integration_tests.log cli_tests.log tui_tests.log quick_tests.log smoke_tests.log; do
        if [ -f "$OUTPUT_DIR/$test_log" ]; then
            echo "### ${test_log%.log}" >> "$report_file"
            echo "\`\`\`" >> "$report_file"
            tail -20 "$OUTPUT_DIR/$test_log" >> "$report_file"
            echo "\`\`\`" >> "$report_file"
            echo "" >> "$report_file"
        fi
    done

    # 添加覆盖率信息
    if [ -f "$OUTPUT_DIR/lcov.info" ]; then
        echo "## 代码覆盖率" >> "$report_file"
        echo "- HTML报告: [查看详情](coverage/index.html)" >> "$report_file"
        echo "- LCOV文件: lcov.info" >> "$report_file"
        echo "" >> "$report_file"
    fi

    log_success "测试报告生成完成: $report_file"
}

# 主函数
main() {
    parse_args "$@"

    if [ "$DOCKER_MODE" = true ]; then
        run_in_docker "$@"
        exit $?
    fi

    # 设置环境
    setup_test_env
    trap cleanup_test_env EXIT

    local exit_code=0

    # 根据测试类型运行相应的测试
    case "$TEST_TYPE" in
        unit)
            run_unit_tests || exit_code=1
            ;;
        integration)
            run_integration_tests || exit_code=1
            ;;
        cli)
            run_cli_tests || exit_code=1
            ;;
        tui)
            run_tui_tests || exit_code=1
            ;;
        performance)
            run_performance_tests || exit_code=1
            ;;
        coverage)
            generate_coverage || exit_code=1
            ;;
        quick)
            run_quick_tests || exit_code=1
            ;;
        smoke)
            run_smoke_tests || exit_code=1
            ;;
        all)
            log_info "运行所有测试..."

            run_unit_tests || exit_code=1
            run_integration_tests || exit_code=1
            run_cli_tests || exit_code=1
            run_tui_tests || exit_code=1
            run_performance_tests || true # 性能测试失败不应该阻止CI
            generate_coverage || true # 覆盖率报告失败不应该阻止CI
            ;;
        *)
            log_error "未知测试类型: $TEST_TYPE"
            show_help
            exit 1
            ;;
    esac

    # 生成报告
    generate_report

    if [ $exit_code -eq 0 ]; then
        log_success "所有测试通过! 🎉"
    else
        log_error "测试失败! 💥"
    fi

    exit $exit_code
}

# 脚本入口
main "$@"