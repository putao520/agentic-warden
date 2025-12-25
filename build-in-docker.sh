#!/bin/bash

# build-in-docker.sh - 在 Docker 容器中编译 agentic-warden
#
# 用途: 跨平台编译静态二进制
# 支持目标: Linux (musl), Windows (MinGW)
# 支持宿主: Linux, macOS, Windows（任何平台都能编译）

set -e

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# 检查 Docker
check_docker() {
    if ! command -v docker &> /dev/null; then
        print_error "Docker 未安装"
        echo "请访问 https://docker.com 安装 Docker"
        exit 1
    fi
    print_success "Docker 已就绪"
}

# 构建 Docker 镜像
build_image() {
    print_info "构建编译环境镜像..."
    docker build -f "$PROJECT_DIR/Dockerfile.build" \
                 -t aiw-builder:latest \
                 "$PROJECT_DIR"
    print_success "镜像构建完成"
}

# 编译指定目标
build_target() {
    local target="$1"

    if [ -z "$target" ]; then
        print_error "未指定编译目标"
        echo "用法: $0 [目标]"
        echo ""
        echo "可用目标:"
        echo "  x86_64-unknown-linux-musl      - Linux x86_64 静态二进制"
        echo "  aarch64-unknown-linux-musl     - Linux ARM64 静态二进制"
        echo "  armv7-unknown-linux-musleabihf - Linux ARMv7 静态二进制"
        echo "  x86_64-pc-windows-gnu          - Windows x86_64 二进制"
        exit 1
    fi

    print_info "编译目标: $target"
    print_info "代码目录: $PROJECT_DIR"

    # Windows 目标不需要 musl 的 RUSTFLAGS
    if [[ "$target" == *"windows"* ]]; then
        docker run --rm \
            -v "$PROJECT_DIR:/workspace" \
            aiw-builder:latest \
            cargo build --release --target "$target"
    else
        docker run --rm \
            -v "$PROJECT_DIR:/workspace" \
            -e "RUSTFLAGS=-C target-feature=+crt-static -C link-self-contained=yes" \
            aiw-builder:latest \
            cargo build --release --target "$target"
    fi

    # 确定输出文件路径（Windows 有 .exe 后缀）
    local output_path="target/$target/release/aiw"
    if [[ "$target" == *"windows"* ]]; then
        output_path="target/$target/release/aiw.exe"
    fi

    if [ -f "$output_path" ]; then
        print_success "编译完成！"
        echo ""
        echo "📦 二进制路径: $output_path"

        # 显示文件信息
        if command -v file &> /dev/null; then
            local file_info=$(file "$output_path")
            echo "📊 文件信息: $file_info"
        fi

        # 显示文件大小
        local size=$(du -h "$output_path" | cut -f1)
        echo "💾 文件大小: $size"

        # 验证静态链接（仅 Linux）
        if [[ "$target" != *"windows"* ]]; then
            if [[ "$file_info" == *"statically linked"* ]]; then
                echo "🔒 状态: 完全静态链接 ✅"
            else
                echo "🔗 状态: 动态链接"
            fi
        else
            echo "🪟 平台: Windows PE32+ 可执行文件"
        fi
    else
        print_error "编译失败，未找到二进制文件"
        exit 1
    fi
}

# 编译所有目标
build_all() {
    print_info "编译所有目标 (Linux + Windows)..."

    local targets=(
        "x86_64-unknown-linux-musl"
        "aarch64-unknown-linux-musl"
        "armv7-unknown-linux-musleabihf"
        "x86_64-pc-windows-gnu"
    )

    for target in "${targets[@]}"; do
        echo ""
        print_info "编译: $target"
        build_target "$target"
    done

    echo ""
    print_success "所有目标编译完成！"
}

# 清理缓存
cleanup() {
    print_info "清理 Docker 容器缓存..."
    docker container prune -f
    print_success "清理完成"
}

# 显示帮助
show_help() {
    cat << EOF
${BLUE}🐳 Agentic-Warden Docker 编译工具${NC}

${YELLOW}用法:${NC}
    $0 [命令] [选项]

${YELLOW}命令:${NC}
    build-image                             构建编译环境镜像
    x86_64-unknown-linux-musl               编译 Linux x86_64 静态二进制
    aarch64-unknown-linux-musl              编译 Linux ARM64 静态二进制
    armv7-unknown-linux-musleabihf          编译 Linux ARMv7 静态二进制
    x86_64-pc-windows-gnu                   编译 Windows x86_64 二进制
    windows                                 编译 Windows x86_64 (简写)
    all                                     编译所有目标 (Linux + Windows)
    shell                                   进入容器交互 shell
    clean                                   清理 Docker 缓存
    help                                    显示此帮助

${YELLOW}示例:${NC}
    # 首次使用：构建镜像
    $0 build-image

    # 编译 Linux x86_64 静态二进制
    $0 x86_64-unknown-linux-musl

    # 编译 Windows x86_64 二进制
    $0 windows

    # 编译所有目标 (Linux + Windows)
    $0 all

    # 进入容器修改和调试
    $0 shell

${YELLOW}功能:${NC}
    ✅ 跨平台编译（Linux, macOS, Windows 都能编译）
    ✅ 完全静态二进制（零运行时依赖）
    ✅ 支持多个编译目标
    ✅ 容器隔离（主机不受影响）
    ✅ 代码热挂载（修改代码即时编译）

${YELLOW}工作原理:${NC}
    1. Docker 镜像包含完整的 Rust + musl-tools
    2. 代码通过 -v 挂载到容器
    3. 在容器内编译（无需主机安装任何工具）
    4. 二进制生成到 target/ 目录

EOF
}

# 进入容器 shell
enter_shell() {
    print_info "进入编译环境 shell..."
    docker run --rm -it \
        -v "$PROJECT_DIR:/workspace" \
        aiw-builder:latest \
        bash
}

# 主程序
main() {
    local cmd="${1:-help}"

    case "$cmd" in
        help|-h|--help)
            show_help
            ;;
        build-image)
            check_docker
            build_image
            ;;
        x86_64-unknown-linux-musl)
            check_docker
            [ ! -f "$PROJECT_DIR/Dockerfile.build" ] && build_image
            build_target "x86_64-unknown-linux-musl"
            ;;
        aarch64-unknown-linux-musl)
            check_docker
            [ ! -f "$PROJECT_DIR/Dockerfile.build" ] && build_image
            build_target "aarch64-unknown-linux-musl"
            ;;
        armv7-unknown-linux-musleabihf)
            check_docker
            [ ! -f "$PROJECT_DIR/Dockerfile.build" ] && build_image
            build_target "armv7-unknown-linux-musleabihf"
            ;;
        x86_64-pc-windows-gnu|windows)
            check_docker
            [ ! -f "$PROJECT_DIR/Dockerfile.build" ] && build_image
            build_target "x86_64-pc-windows-gnu"
            ;;
        all)
            check_docker
            [ ! -f "$PROJECT_DIR/Dockerfile.build" ] && build_image
            build_all
            ;;
        shell)
            check_docker
            [ ! -f "$PROJECT_DIR/Dockerfile.build" ] && build_image
            enter_shell
            ;;
        clean)
            cleanup
            ;;
        *)
            print_error "未知命令: $cmd"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# 执行主程序
main "$@"
