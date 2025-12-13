#!/bin/bash

# Universal Release Manager - 通用发布版本管理器
# 支持多编程语言的包管理器自动版本同步和发布

set -euo pipefail

# 脚本配置
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="$PROJECT_ROOT/.release-config.yml"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
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
Universal Release Manager - 通用发布版本管理器

用法: $0 [VERSION] [OPTIONS] [RELEASE_NOTES]

参数:
  VERSION           新版本号 (如: 1.0.0, 0.4.8, 2.1.3-beta)

选项:
  -h, --help        显示帮助信息
  -i, --interactive 交互式模式
  -d, --dry-run     预演模式，不执行实际操作
  -c, --config      指定配置文件路径 (默认: .release-config.yml)
  -a, --auto-changelog 自动生成CHANGELOG
  -t, --tag-only    仅创建Git标签，不更新配置文件
  -f, --force       强制执行，跳过验证
  -r, --rollback ROLLBACK_VERSION  回滚到指定版本
  --skip-tests      跳过测试检查
  --skip-git        跳过Git操作
  --verbose         详细输出

示例:
  $0 1.0.0 "重大功能更新"
  $0 0.4.8 --auto-changelog "修复bug和添加新功能"
  $0 --interactive
  $0 --dry-run 0.4.8
  $0 --rollback 0.4.7

支持的包管理器:
  Rust (Cargo), Node.js (npm/yarn/pnpm), Python (pip/poetry),
  Go (modules), Java (Maven/Gradle), Ruby (Gems), PHP (Composer)

EOF
}

# 解析命令行参数
parse_arguments() {
    VERSION=""
    RELEASE_NOTES=""
    INTERACTIVE=false
    DRY_RUN=false
    CONFIG_FILE="$PROJECT_ROOT/.release-config.yml"
    AUTO_CHANGELOG=false
    TAG_ONLY=false
    FORCE=false
    ROLLBACK_VERSION=""
    SKIP_TESTS=false
    SKIP_GIT=false
    VERBOSE=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -i|--interactive)
                INTERACTIVE=true
                shift
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -c|--config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            -a|--auto-changelog)
                AUTO_CHANGELOG=true
                shift
                ;;
            -t|--tag-only)
                TAG_ONLY=true
                shift
                ;;
            -f|--force)
                FORCE=true
                shift
                ;;
            -r|--rollback)
                ROLLBACK_VERSION="$2"
                shift 2
                ;;
            --skip-tests)
                SKIP_TESTS=true
                shift
                ;;
            --skip-git)
                SKIP_GIT=true
                shift
                ;;
            --verbose)
                VERBOSE=true
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

    # 交互式模式
    if [[ "$INTERACTIVE" == true ]]; then
        interactive_mode
    fi

    # 回滚模式
    if [[ -n "$ROLLBACK_VERSION" ]]; then
        rollback_version "$ROLLBACK_VERSION"
        exit 0
    fi

    # 验证必需参数
    if [[ -z "$VERSION" ]]; then
        log_error "请指定版本号或使用 --interactive 模式"
        show_help
        exit 1
    fi

    # 验证版本号格式
    if [[ "$FORCE" != true ]] && ! validate_version_format "$VERSION"; then
        log_error "版本号格式错误，请使用 semantic versioning (如: 1.0.0, 0.4.8)"
        exit 1
    fi
}

# 交互式模式
interactive_mode() {
    log_info "进入交互式模式"

    # 检测当前项目信息
    detect_project_info

    # 获取当前版本
    local current_version=$(get_current_version)
    log_info "当前版本: $current_version"

    # 输入新版本号
    echo
    read -p "请输入新版本号 (当前: $current_version): " input_version
    if [[ -n "$input_version" ]]; then
        VERSION="$input_version"
    else
        log_error "版本号不能为空"
        exit 1
    fi

    # 验证版本号格式
    if ! validate_version_format "$VERSION"; then
        log_error "版本号格式错误"
        exit 1
    fi

    # 输入发布说明
    echo
    read -p "请输入发布说明 (可选): " input_notes
    RELEASE_NOTES="$input_notes"

    # 确认选项
    echo
    echo "发布配置:"
    echo "  版本号: $VERSION"
    echo "  发布说明: ${RELEASE_NOTES:-无}"
    echo "  项目类型: $PROJECT_TYPE"
    echo "  包管理器: ${DETECTED_MANAGERS[*]}"
    echo

    read -p "确认发布? (y/N): " confirm
    if [[ "$confirm" =~ ^[Yy]$ ]]; then
        log_info "开始发布流程..."
    else
        log_info "取消发布"
        exit 0
    fi
}

# 检测项目信息
detect_project_info() {
    cd "$PROJECT_ROOT"

    DETECTED_MANAGERS=()
    PROJECT_TYPE="unknown"

    # 检测各种包管理器
    if [[ -f "Cargo.toml" ]]; then
        DETECTED_MANAGERS+=("cargo")
        PROJECT_TYPE="rust"
        log_info "检测到 Rust Cargo 项目"
    fi

    if [[ -f "package.json" ]]; then
        DETECTED_MANAGERS+=("npm")
        if [[ "$PROJECT_TYPE" == "unknown" ]]; then
            PROJECT_TYPE="nodejs"
        fi
        log_info "检测到 Node.js 项目"
    fi

    if [[ -f "pyproject.toml" ]] || [[ -f "setup.py" ]]; then
        DETECTED_MANAGERS+=("python")
        if [[ "$PROJECT_TYPE" == "unknown" ]]; then
            PROJECT_TYPE="python"
        fi
        log_info "检测到 Python 项目"
    fi

    if [[ -f "go.mod" ]]; then
        DETECTED_MANAGERS+=("go")
        if [[ "$PROJECT_TYPE" == "unknown" ]]; then
            PROJECT_TYPE="go"
        fi
        log_info "检测到 Go 项目"
    fi

    if [[ -f "pom.xml" ]] || [[ -f "build.gradle" ]]; then
        DETECTED_MANAGERS+=("java")
        if [[ "$PROJECT_TYPE" == "unknown" ]]; then
            PROJECT_TYPE="java"
        fi
        log_info "检测到 Java 项目"
    fi

    if [[ -f "Gemfile" ]] || [[ -f "*.gemspec" ]]; then
        DETECTED_MANAGERS+=("ruby")
        if [[ "$PROJECT_TYPE" == "unknown" ]]; then
            PROJECT_TYPE="ruby"
        fi
        log_info "检测到 Ruby 项目"
    fi

    if [[ -f "composer.json" ]]; then
        DETECTED_MANAGERS+=("php")
        if [[ "$PROJECT_TYPE" == "unknown" ]]; then
            PROJECT_TYPE="php"
        fi
        log_info "检测到 PHP 项目"
    fi

    if [[ -f "Dockerfile" ]]; then
        DETECTED_MANAGERS+=("docker")
        log_info "检测到 Docker 项目"
    fi

    if [[ "$PROJECT_TYPE" == "unknown" ]]; then
        log_warning "未检测到支持的包管理器"
        exit 1
    fi

    log_info "项目类型: $PROJECT_TYPE"
    log_info "检测到的包管理器: ${DETECTED_MANAGERS[*]}"
}

# 获取当前版本号
get_current_version() {
    cd "$PROJECT_ROOT"

    # 优先从 Cargo.toml 获取
    if [[ -f "Cargo.toml" ]]; then
        grep '^version = ' Cargo.toml | sed 's/version = "//g' | sed 's/"//g' | tr -d ' '
        return
    fi

    # 从 package.json 获取
    if [[ -f "package.json" ]]; then
        grep '"version"' package.json | sed 's/.*"version": "//g' | sed 's/".*//g' | tr -d ' '
        return
    fi

    # 从 pyproject.toml 获取
    if [[ -f "pyproject.toml" ]]; then
        grep '^version = ' pyproject.toml | sed 's/version = "//g' | sed 's/"//g' | tr -d ' '
        return
    fi

    # 从 go.mod 获取
    if [[ -f "go.mod" ]]; then
        # Go modules 没有统一的版本字段，返回默认值
        echo "0.0.0"
        return
    fi

    echo "0.0.0"
}

# 验证版本号格式 (semantic versioning)
validate_version_format() {
    local version=$1

    # 基本的语义化版本正则表达式
    if [[ $version =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-([0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*))?(\+([0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*))?$ ]]; then
        return 0
    else
        return 1
    fi
}

# 验证版本号递增
validate_version_increment() {
    local current_version=$1
    local new_version=$2

    # 简单的版本号比较
    if [[ "$current_version" == "$new_version" ]]; then
        log_error "新版本号与当前版本号相同"
        return 1
    fi

    # 可以添加更复杂的版本号递增验证逻辑
    return 0
}

# 更新 Cargo.toml 版本
update_cargo_version() {
    local new_version=$1
    local cargo_file="$PROJECT_ROOT/Cargo.toml"

    if [[ -f "$cargo_file" ]]; then
        log_info "更新 Cargo.toml 版本: $new_version"

        if [[ "$DRY_RUN" != true ]]; then
            # 使用 sed 更新版本号
            if [[ "$VERBOSE" == true ]]; then
                log_info "执行: sed -i 's/^version = .*/version = \"$new_version\"/' $cargo_file"
            fi

            if command -v sd >/dev/null 2>&1; then
                sd 'version = ".+?""version = "'$new_version'" "$cargo_file"
            else
                sed -i.bak 's/^version = .*/version = "'$new_version'"/' "$cargo_file"
                rm -f "$cargo_file.bak"
            fi
        fi
    fi
}

# 更新 package.json 版本
update_npm_version() {
    local new_version=$1
    local package_file="$PROJECT_ROOT/package.json"

    if [[ -f "$package_file" ]]; then
        log_info "更新 package.json 版本: $new_version"

        if [[ "$DRY_RUN" != true ]]; then
            # 使用 npm version 命令（如果可用）
            if command -v npm >/dev/null 2>&1; then
                npm version "$new_version" --no-git-tag-version
            else
                # 手动更新 JSON
                if command -v jq >/dev/null 2>&1; then
                    jq '.version = "'$new_version'" "$package_file" > "$package_file.tmp" && mv "$package_file.tmp" "$package_file"
                else
                    log_warning "未找到 jq 或 npm，跳过 package.json 更新"
                fi
            fi
        fi
    fi

    # 同时更新 npm-package 目录
    local npm_package_file="$PROJECT_ROOT/npm-package/package.json"
    if [[ -f "$npm_package_file" ]]; then
        log_info "更新 npm-package/package.json 版本: $new_version"

        if [[ "$DRY_RUN" != true ]]; then
            if command -v jq >/dev/null 2>&1; then
                jq '.version = "'$new_version'" "$npm_package_file" > "$npm_package_file.tmp" && mv "$npm_package_file.tmp" "$npm_package_file"
            fi
        fi
    fi
}

# 更新 Python 版本
update_python_version() {
    local new_version=$1

    # 更新 pyproject.toml
    local pyproject_file="$PROJECT_ROOT/pyproject.toml"
    if [[ -f "$pyproject_file" ]]; then
        log_info "更新 pyproject.toml 版本: $new_version"

        if [[ "$DRY_RUN" != true ]]; then
            if command -v sd >/dev/null 2>&1; then
                sd 'version = ".+?""version = "'$new_version'" "$pyproject_file"
            else
                sed -i.bak 's/^version = .*/version = "'$new_version'"/' "$pyproject_file"
                rm -f "$pyproject_file.bak"
            fi
        fi
    fi

    # 更新 setup.py
    local setup_file="$PROJECT_ROOT/setup.py"
    if [[ -f "$setup_file" ]]; then
        log_info "更新 setup.py 版本: $new_version"

        if [[ "$DRY_RUN" != true ]]; then
            if command -v sd >/dev/null 2>&1; then
                sd "version=('.+?')\"version='$new_version'" "$setup_file"
                sd 'version = ".+?""version = "'$new_version'" "$setup_file"
            else
                sed -i.bak "s/version=.*/version='$new_version'/" "$setup_file"
                sed -i.bak 's/version = .*/version = "'$new_version'"/' "$setup_file"
                rm -f "$setup_file.bak"
            fi
        fi
    fi
}

# 更新 Go 版本（Go modules 不需要版本号更新）
update_go_version() {
    log_info "Go modules 不需要版本号更新"
}

# 更新 Java 版本
update_java_version() {
    local new_version=$1

    # 更新 pom.xml
    local pom_file="$PROJECT_ROOT/pom.xml"
    if [[ -f "$pom_file" ]]; then
        log_info "更新 pom.xml 版本: $new_version"

        if [[ "$DRY_RUN" != true ]]; then
            # 使用 Maven 更新版本
            if command -v mvn >/dev/null 2>&1; then
                mvn versions:set -DnewVersion="$new_version"
            else
                # 手动更新 XML
                if command -v sd >/dev/null 2>&1; then
                    sd '<version>.+?</version><version>'$new_version'</version>' "$pom_file"
                else
                    sed -i.bak 's/<version>.*<\/version>/<version>'$new_version'<\/version>/' "$pom_file"
                    rm -f "$pom_file.bak"
                fi
            fi
        fi
    fi

    # 更新 build.gradle
    local gradle_file="$PROJECT_ROOT/build.gradle"
    if [[ -f "$gradle_file" ]]; then
        log_info "更新 build.gradle 版本: $new_version"

        if [[ "$DRY_RUN" != true ]]; then
            if command -v sd >/dev/null 2>&1; then
                sd "version = '.+?'\"version = '$new_version'" "$gradle_file"
                sd 'version = ".+?""version = "'$new_version'" "$gradle_file"
            fi
        fi
    fi
}

# 更新 Ruby 版本
update_ruby_version() {
    local new_version=$1

    # 更新 *.gemspec 文件
    for gemspec_file in "$PROJECT_ROOT"/*.gemspec; do
        if [[ -f "$gemspec_file" ]]; then
            log_info "更新 $(basename "$gemspec_file") 版本: $new_version"

            if [[ "$DRY_RUN" != true ]]; then
                if command -v sd >/dev/null 2>&1; then
                    sd 's\.version = .+?.version = "'$new_version'" "$gemspec_file"
                else
                    sed -i.bak 's/s\.version = .*/s.version = "'$new_version'"/' "$gemspec_file"
                    rm -f "$gemspec_file.bak"
                fi
            fi
        fi
    done
}

# 更新 PHP 版本
update_php_version() {
    local new_version=$1
    local composer_file="$PROJECT_ROOT/composer.json"

    if [[ -f "$composer_file" ]]; then
        log_info "更新 composer.json 版本: $new_version"

        if [[ "$DRY_RUN" != true ]]; then
            if command -v composer >/dev/null 2>&1; then
                composer config version "$new_version"
            elif command -v jq >/dev/null 2>&1; then
                jq '.version = "'$new_version'" "$composer_file" > "$composer_file.tmp" && mv "$composer_file.tmp" "$composer_file"
            fi
        fi
    fi
}

# 更新 Docker 版本
update_docker_version() {
    log_info "Docker 版本更新通常通过标签管理，配置文件无需更新"

    # 可以在这里添加 Dockerfile 中的版本号更新逻辑
    local dockerfile="$PROJECT_ROOT/Dockerfile"
    if [[ -f "$dockerfile" ]]; then
        if grep -q "LABEL version" "$dockerfile"; then
            log_info "更新 Dockerfile 版本标签: $new_version"

            if [[ "$DRY_RUN" != true ]]; then
                if command -v sd >/dev/null 2>&1; then
                    sd 'LABEL version ".+?""LABEL version "'$new_version'" "$dockerfile"
                else
                    sed -i.bak 's/LABEL version .*/LABEL version "'$new_version'"/' "$dockerfile"
                    rm -f "$dockerfile.bak"
                fi
            fi
        fi
    fi
}

# 更新所有版本文件
update_all_versions() {
    local new_version=$1

    log_info "开始更新所有配置文件的版本号到: $new_version"

    # 根据检测到的包管理器更新相应的文件
    for manager in "${DETECTED_MANAGERS[@]}"; do
        case $manager in
            "cargo")
                update_cargo_version "$new_version"
                ;;
            "npm")
                update_npm_version "$new_version"
                ;;
            "python")
                update_python_version "$new_version"
                ;;
            "go")
                update_go_version "$new_version"
                ;;
            "java")
                update_java_version "$new_version"
                ;;
            "ruby")
                update_ruby_version "$new_version"
                ;;
            "php")
                update_php_version "$new_version"
                ;;
            "docker")
                update_docker_version "$new_version"
                ;;
            *)
                log_warning "未知的包管理器: $manager"
                ;;
        esac
    done

    log_success "所有版本文件更新完成"
}

# 运行测试
run_tests() {
    if [[ "$SKIP_TESTS" == true ]]; then
        log_info "跳过测试检查"
        return 0
    fi

    log_info "运行测试检查..."

    cd "$PROJECT_ROOT"

    # 根据项目类型运行相应的测试
    if [[ "$PROJECT_TYPE" == "rust" ]] && command -v cargo >/dev/null 2>&1; then
        if cargo test --quiet; then
            log_success "Rust 测试通过"
        else
            log_error "Rust 测试失败，发布中止"
            return 1
        fi
    elif [[ "$PROJECT_TYPE" == "nodejs" ]] && command -v npm >/dev/null 2>&1; then
        if npm test --silent; then
            log_success "Node.js 测试通过"
        else
            log_warning "Node.js 测试失败，继续发布"
        fi
    elif [[ "$PROJECT_TYPE" == "python" ]] && command -v python >/dev/null 2>&1; then
        if python -m pytest --quiet 2>/dev/null; then
            log_success "Python 测试通过"
        else
            log_warning "Python 测试失败，继续发布"
        fi
    else
        log_warning "未找到相应的测试工具，跳过测试检查"
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

    # 检查是否有未提交的更改
    if [[ -n "$(git status --porcelain)" ]]; then
        log_warning "检测到未提交的更改，建议先提交或储藏"
        if [[ "$FORCE" != true ]]; then
            read -p "是否继续发布? (y/N): " confirm
            if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
                log_info "取消发布"
                exit 0
            fi
        fi
    fi

    # 添加版本文件到 Git
    log_info "添加版本更新到 Git"
    git add .

    # 创建提交
    local commit_message="Release v$VERSION"
    if [[ -n "$RELEASE_NOTES" ]]; then
        commit_message="$commit_message

$RELEASE_NOTES"
    fi

    if [[ "$DRY_RUN" != true ]]; then
        git commit -m "$commit_message"
        log_success "Git 提交创建完成"
    fi

    # 创建标签
    local tag_name="v$VERSION"
    log_info "创建 Git 标签: $tag_name"

    if [[ "$DRY_RUN" != true ]]; then
        git tag -a "$tag_name" -m "Release $tag_name

$RELEASE_NOTES"
        log_success "Git 标签创建完成"
    fi

    # 推送到远程仓库
    if [[ "$DRY_RUN" != true ]]; then
        log_info "推送到远程仓库"
        git push origin main
        git push origin "$tag_name"
        log_success "推送完成"
    fi
}

# 生成 CHANGELOG
generate_changelog() {
    if [[ "$AUTO_CHANGELOG" != true ]]; then
        return 0
    fi

    log_info "生成 CHANGELOG"

    cd "$PROJECT_ROOT"

    # 获取上一个版本标签
    local prev_tag=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")

    local changelog_file="$PROJECT_ROOT/CHANGELOG.md"

    # 创建或追加 CHANGELOG
    {
        echo "## [$VERSION] - $(date +%Y-%m-%d)"
        echo ""

        if [[ -n "$RELEASE_NOTES" ]]; then
            echo "### 主要更新"
            echo "$RELEASE_NOTES"
            echo ""
        fi

        if [[ -n "$prev_tag" ]]; then
            echo "### 变更详情"
            echo ""
            if git log --pretty=format:"- %s (%h)" "$prev_tag"..HEAD; then
                echo ""
            fi
        fi

        echo "---"
        echo ""
    } >> "$changelog_file"

    log_success "CHANGELOG 更新完成"
}

# 触发发布流程
trigger_release_workflow() {
    log_info "发布流程已触发"
    log_info "GitHub Actions 将自动处理后续的发布步骤"

    if [[ "$PROJECT_TYPE" == "rust" ]]; then
        log_info "Rust 项目: crates.io 发布将在 CI 中执行"
    elif [[ "$PROJECT_TYPE" == "nodejs" ]]; then
        log_info "Node.js 项目: npm publish 将在 CI 中执行"
    fi
}

# 回滚版本
rollback_version() {
    local target_version=$1

    log_info "开始回滚到版本: $target_version"

    cd "$PROJECT_ROOT"

    # 检查目标标签是否存在
    if ! git rev-parse "refs/tags/v$target_version" >/dev/null 2>&1; then
        log_error "目标版本标签 v$target_version 不存在"
        exit 1
    fi

    # 确认回滚操作
    read -p "确认回滚到版本 v$target_version? 这将重置当前分支到该标签 (y/N): " confirm
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        log_info "取消回滚"
        exit 0
    fi

    # 重置到目标标签
    git reset --hard "v$target_version"

    # 强制推送（谨慎操作）
    read -p "确认强制推送到远程仓库? 这将覆盖远程历史 (y/N): " confirm_push
    if [[ "$confirm_push" =~ ^[Yy]$ ]]; then
        git push --force-with-lease origin main
    fi

    log_success "回滚完成"
}

# 主发布流程
main() {
    log_info "Universal Release Manager v1.0.0"
    log_info "项目路径: $PROJECT_ROOT"

    # 解析命令行参数
    parse_arguments "$@"

    # 检测项目信息
    detect_project_info

    # 获取当前版本并验证
    local current_version=$(get_current_version)
    log_info "当前版本: $current_version"
    log_info "目标版本: $VERSION"

    if [[ "$FORCE" != true ]] && ! validate_version_increment "$current_version" "$VERSION"; then
        exit 1
    fi

    # 预演模式
    if [[ "$DRY_RUN" == true ]]; then
        log_info "=== 预演模式 - 不会执行实际操作 ==="
        log_info "将要更新的包管理器: ${DETECTED_MANAGERS[*]}"
        log_info "版本号变更: $current_version -> $VERSION"
        log_info "=== 预演模式结束 ==="
        exit 0
    fi

    # 执行发布流程
    log_info "开始发布流程 v$VERSION"

    # 1. 运行测试
    if ! run_tests; then
        exit 1
    fi

    # 2. 更新版本号（除非是仅标签模式）
    if [[ "$TAG_ONLY" != true ]]; then
        update_all_versions "$VERSION"
    fi

    # 3. 生成 CHANGELOG
    generate_changelog

    # 4. 创建 Git 提交和标签
    create_git_commit_and_tag

    # 5. 触发发布流程
    trigger_release_workflow

    log_success "🎉 发布 v$VERSION 完成！"
    log_info "查看发布详情: https://github.com/putao520/agentic-warden/releases/tag/v$VERSION"
}

# 脚本入口点
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi