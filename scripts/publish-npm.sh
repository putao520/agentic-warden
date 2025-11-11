#!/usr/bin/env bash

# 手动发布 NPM 包脚本
# 使用方法：./scripts/publish-npm.sh [dry-run|publish]

set -e

MODE=${1:-"dry-run"}

echo "=========================================="
echo "Agentic Warden NPM 发布工具"
echo "=========================================="
echo ""

# 1. 验证环境
echo "📋 步骤 1: 验证环境"
echo "----------------------------------------"

if ! command -v node &> /dev/null; then
    echo "❌ Node.js 未安装，请先安装 Node.js"
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo "❌ NPM 未安装，请先安装 NPM"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo 未安装，请先安装 Rust"
    exit 1
fi

NODE_VERSION=$(node --version)
NPM_VERSION=$(npm --version)
RUST_VERSION=$(rustc --version)

echo "  ✓ Node.js: $NODE_VERSION"
echo "  ✓ NPM: $NPM_VERSION"
echo "  ✓ Rust: $RUST_VERSION"
echo ""

# 2. 验证 Git 状态
echo "📋 步骤 2: 验证 Git 状态"
echo "----------------------------------------"

if ! git diff-index --quiet HEAD --; then
    echo "⚠️  警告：工作区有未提交的更改"
    git status --short
    echo ""
    read -p "是否继续？(y/N): " -n 1 -r
echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "已取消"
        exit 1
    fi
else
    echo "  ✓ 工作区干净"
fi
echo ""

# 3. 获取版本
echo "📋 步骤 3: 获取版本信息"
echo "----------------------------------------"

CARGO_VERSION=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/' | tr -d '"')
PACKAGE_VERSION=$(node -p "require('./package.json').version")

echo "  Cargo.toml:  $CARGO_VERSION"
echo "  package.json: $PACKAGE_VERSION"

if [ "$CARGO_VERSION" != "$PACKAGE_VERSION" ]; then
    echo ""
    echo "⚠️  警告：版本不一致"
    echo ""
    read -p "是否自动同步版本号？(y/N): " -n 1 -r
echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # 更新 package.json
        npm version --allow-same-version --no-git-tag-version "$CARGO_VERSION"
        echo "  ✓ 已同步版本为: $CARGO_VERSION"
    else
        echo ""
        echo "请手动同步版本号后再试"
        exit 1
    fi
fi
echo ""

# 4. 构建项目
echo "📋 步骤 4: 构建 Rust 项目"
echo "----------------------------------------"

echo "  正在构建 release 版本..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ 构建失败"
    exit 1
fi

echo "  ✓ 构建成功"
echo ""

# 5. 配置 NPM
echo "📋 步骤 5: 配置 NPM"
echo "----------------------------------------"

# 检查是否登录
NPM_USER=$(npm whoami 2>/dev/null || echo "")

if [ -z "$NPM_USER" ]; then
    echo "  🔐 未登录到 NPM"
    echo ""
    echo "请使用以下命令登录:"
    echo "  npm login"
    echo ""
    read -p "登录完成后按 Enter 继续..."
fi

NPM_USER=$(npm whoami)
echo "  ✓ NPM 用户: $NPM_USER"
echo ""

# 6. 验证包文件
echo "📋 步骤 6: 验证 NPM 包"
echo "----------------------------------------"

# 检查必要文件
for file in package.json bin/install.js bin/agentic-warden.js .npmignore; do
    if [ ! -f "$file" ]; then
        echo "  ❌ 缺少文件: $file"
        exit 1
    fi
done

echo "  ✓ package.json: 存在"
echo "  ✓ bin/install.js: 存在"
echo "  ✓ bin/agentic-warden.js: 存在"
echo "  ✓ .npmignore: 存在"
echo ""

# 7. 准备二进制文件
echo "📋 步骤 7: 准备二进制文件"
echo "----------------------------------------"

# 当前平台
PLATFORM=$(node -e "
const os = require('os');
const platform = os.platform();
const arch = os.arch();
if (platform === 'linux' && arch === 'x64') console.log('x86_64-unknown-linux-gnu');
else if (platform === 'linux' && arch === 'arm64') console.log('aarch64-unknown-linux-gnu');
else if (platform === 'darwin' && arch === 'x64') console.log('x86_64-apple-darwin');
else if (platform === 'darwin' && arch === 'arm64') console.log('aarch64-apple-darwin');
else if (platform === 'win32' && arch === 'x64') console.log('x86_64-pc-windows-msvc');
else console.log('unknown');
")

echo "  当前平台: $PLATFORM"

# 创建目录
mkdir -p "binaries/$PLATFORM"

# 复制二进制文件
if [ -f "target/release/agentic-warden" ]; then
    cp target/release/agentic-warden "binaries/$PLATFORM/"
    chmod +x "binaries/$PLATFORM/agentic-warden"
    echo "  ✓ 二进制文件已复制: binaries/$PLATFORM/agentic-warden"
elif [ -f "target/release/agentic-warden.exe" ]; then
    cp target/release/agentic-warden.exe "binaries/$PLATFORM/"
    echo "  ✓ 二进制文件已复制: binaries/$PLATFORM/agentic-warden.exe"
else
    echo "  ❌ 未找到二进制文件"
    exit 1
fi

echo ""

# 8. 预览包内容
echo "📋 步骤 8: 预览 NPM 包内容"
echo "----------------------------------------"

npm pack --dry-run

echo ""
echo "📦 发布内容:"
echo "  版本: $PACKAGE_VERSION"
echo "  平台: $PLATFORM"
echo "  二进制: binaries/$PLATFORM/"
echo ""

# 9. 发布
echo "=========================================="
echo "🚀 准备发布"
echo "=========================================="
echo ""

if [ "$MODE" = "publish" ]; then
    read -p "确认发布到 NPM？(y/N): " -n 1 -r
echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "  正在发布..."
        npm publish --access public

        if [ $? -eq 0 ]; then
            echo ""
            echo "  ✅ 发布成功！"
            echo ""
            echo "  用户现在可以使用:"
            echo "    npx agentic-warden --help"
            echo ""
        else
            echo ""
            echo "  ❌ 发布失败"
            exit 1
        fi
    else
        echo "  已取消发布"
    fi
else
    echo "  💡 当前为 dry-run 模式"
    echo ""
    echo "  要实际发布，请运行:"
    echo "    ./scripts/publish-npm.sh publish"
    echo ""
fi

echo "=========================================="
echo "完成"
echo "=========================================="
