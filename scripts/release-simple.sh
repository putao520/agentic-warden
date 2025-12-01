#!/bin/bash

# 简化版发布管理器 - 专注于解决当前项目的版本同步问题

set -euo pipefail

# 脚本配置
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 显示帮助信息
show_help() {
    cat << EOF
简化版发布管理器

用法: $0 <VERSION> [RELEASE_NOTES]

参数:
  VERSION           新版本号 (如: 1.0.0, 0.4.8)
  RELEASE_NOTES      发布说明 (可选)

选项:
  -h, --help        显示帮助信息
  --dry-run         预演模式，不执行实际操作
  --skip-git        跳过Git操作

示例:
  $0 0.4.8 "添加交互式AI CLI启动功能"
  $0 1.0.0 "重大功能更新"
  $0 --dry-run 0.4.8

EOF
}

# 解析命令行参数
parse_arguments() {
    VERSION=""
    RELEASE_NOTES=""
    DRY_RUN=false
    SKIP_GIT=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --skip-git)
                SKIP_GIT=true
                shift
                ;;
            -*)
                log_error "未知选项: $1"
                show_help
                exit 1
                ;;
            *)
                if [[ -z "$VERSION" ]]; then
                    VERSION="$1"
                elif [[ -z "$RELEASE_NOTES" ]]; then
                    RELEASE_NOTES="$1"
                else
                    log_error "多余的参数: $1"
                    show_help
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # 验证必需参数
    if [[ -z "$VERSION" ]]; then
        log_error "请指定版本号"
        show_help
        exit 1
    fi

    # 验证版本号格式
    if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+)?$ ]]; then
        log_error "版本号格式错误，请使用 semantic versioning (如: 1.0.0, 0.4.8)"
        exit 1
    fi
}

# 获取当前版本号
get_current_version() {
    # 从 Cargo.toml 获取
    if [[ -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        grep '^version = ' "$PROJECT_ROOT/Cargo.toml" | sed 's/version = "//g' | sed 's/"//g' | tr -d ' '
        return
    fi

    # 从 package.json 获取
    if [[ -f "$PROJECT_ROOT/package.json" ]]; then
        grep '"version"' "$PROJECT_ROOT/package.json" | sed 's/.*"version": "//g' | sed 's/".*//g' | tr -d ' '
        return
    fi

    echo "0.0.0"
}

# 更新 Cargo.toml 版本
update_cargo_version() {
    local cargo_file="$PROJECT_ROOT/Cargo.toml"

    if [[ -f "$cargo_file" ]]; then
        log_info "更新 Cargo.toml 版本: $VERSION"

        if [[ "$DRY_RUN" != true ]]; then
            # 备份原文件
            cp "$cargo_file" "$cargo_file.bak"

            # 使用 sed 更新版本号
            sed -i 's/^version = .*/version = "'$VERSION'"/' "$cargo_file"

            # 验证更新是否成功
            local updated_version=$(grep '^version = ' "$cargo_file" | sed 's/version = "//g' | sed 's/"//g' | tr -d ' ')
            if [[ "$updated_version" == "$VERSION" ]]; then
                log_success "Cargo.toml 更新成功"
                rm -f "$cargo_file.bak"
            else
                log_error "Cargo.toml 更新失败，恢复备份"
                mv "$cargo_file.bak" "$cargo_file"
                return 1
            fi
        else
            log_info "[预演] 将更新 Cargo.toml 版本到: $VERSION"
        fi
    fi
}

# 更新 package.json 版本
update_npm_version() {
    local package_file="$PROJECT_ROOT/package.json"

    if [[ -f "$package_file" ]]; then
        log_info "更新 package.json 版本: $VERSION"

        if [[ "$DRY_RUN" != true ]]; then
            # 备份原文件
            cp "$package_file" "$package_file.bak"

            # 使用 sed 更新版本号
            sed -i 's/"version": "[^"]*"/"version": "'$VERSION'"/' "$package_file"

            # 验证更新是否成功
            local updated_version=$(grep '"version"' "$package_file" | sed 's/.*"version": "//g' | sed 's/".*//g' | tr -d ' ')
            if [[ "$updated_version" == "$VERSION" ]]; then
                log_success "package.json 更新成功"
                rm -f "$package_file.bak"
            else
                log_error "package.json 更新失败，恢复备份"
                mv "$package_file.bak" "$package_file"
                return 1
            fi
        else
            log_info "[预演] 将更新 package.json 版本到: $VERSION"
        fi
    fi
}

# 创建 Git 提交和标签
create_git_commit_and_tag() {
    if [[ "$SKIP_GIT" == true ]]; then
        log_info "跳过 Git 操作"
        return 0
    fi

    cd "$PROJECT_ROOT"

    # 检查 Git 仓库状态
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        log_error "当前目录不是 Git 仓库"
        return 1
    fi

    # 添加版本文件到 Git
    log_info "添加版本更新到 Git"
    if [[ "$DRY_RUN" != true ]]; then
        git add .
        git diff --cached --name-only
    fi

    # 创建提交
    local commit_message="Release v$VERSION"
    if [[ -n "$RELEASE_NOTES" ]]; then
        commit_message="$commit_message

$RELEASE_NOTES"
    fi

    if [[ "$DRY_RUN" != true ]]; then
        git commit -m "$commit_message"
        log_success "Git 提交创建完成"
    else
        log_info "[预演] 将创建提交: $commit_message"
    fi

    # 创建标签
    local tag_name="v$VERSION"
    log_info "创建 Git 标签: $tag_name"

    if [[ "$DRY_RUN" != true ]]; then
        git tag -a "$tag_name" -m "Release $tag_name

$RELEASE_NOTES"
        log_success "Git 标签创建完成"
    else
        log_info "[预演] 将创建标签: $tag_name"
    fi

    # 推送到远程仓库
    if [[ "$DRY_RUN" != true ]]; then
        log_info "推送到远程仓库"
        git push origin main
        git push origin "$tag_name"
        log_success "推送完成"
    else
        log_info "[预演] 将推送到远程仓库"
    fi
}

# 主发布流程
main() {
    log_info "简化版发布管理器"
    log_info "项目路径: $PROJECT_ROOT"

    # 解析命令行参数
    parse_arguments "$@"

    # 获取当前版本
    local current_version=$(get_current_version)
    log_info "当前版本: $current_version"
    log_info "目标版本: $VERSION"

    if [[ "$current_version" == "$VERSION" ]]; then
        log_error "新版本号与当前版本号相同"
        exit 1
    fi

    # 预演模式
    if [[ "$DRY_RUN" == true ]]; then
        log_info "=== 预演模式 - 不会执行实际操作 ==="
        update_cargo_version
        update_npm_version
        create_git_commit_and_tag
        log_info "=== 预演模式结束 ==="
        exit 0
    fi

    # 执行发布流程
    log_info "开始发布流程 v$VERSION"

    # 1. 更新版本号
    if ! update_cargo_version; then
        exit 1
    fi

    if ! update_npm_version; then
        exit 1
    fi

    # 2. 创建 Git 提交和标签
    create_git_commit_and_tag

    log_success "🎉 发布 v$VERSION 完成！"
    log_info "查看发布详情: https://github.com/putao520/agentic-warden/releases/tag/v$VERSION"
}

# 脚本入口点
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi