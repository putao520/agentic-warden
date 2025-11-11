#!/usr/bin/env bash

# 发布占位 NPM 包脚本
# 使用方法:
#   ./publish-placeholder.sh    # 检查环境
#   ./publish-placeholder.sh publish  # 发布占位包

set -e

COMMAND=${1:-"check"}

echo "=========================================="
echo "Agentic Warden - 占位包发布工具"
echo "=========================================="
echo ""

# 备份原始 package.json
if [ -f "package.json" ] && [ ! -L "package.json" ]; then
    echo "📦 备份原始 package.json..."
    cp package.json package.json.backup
    echo ""
fi

# 使用占位文件
echo "📋 准备占位文件..."
cp package-placeholder.json package.json
cp .npmignore-placeholder .npmignore
echo "  ✓ 已切换到占位配置"
echo ""

case "$COMMAND" in
    check)
        echo "📋 检查发布环境..."
        echo ""

        # 检查 Node.js
        if ! command -v node &> /dev/null; then
            echo "❌ Node.js 未安装"
            exit 1
        fi
        echo "✓ Node.js: $(node --version)"

        # 检查 NPM
        if ! command -v npm &> /dev/null; then
            echo "❌ NPM 未安装"
            exit 1
        fi
        echo "✓ NPM: $(npm --version)"

        # 检查文件
        for file in package.json bin-placeholder.js README-placeholder.md; do
            if [ ! -f "$file" ]; then
                echo "❌ 缺少文件: $file"
                exit 1
            fi
        done
        echo "✓ 所有必要文件存在"

        # 预览包内容
        echo ""
        echo "📦 包内容预览:"
        npm pack --dry-run 2>/dev/null | tail -20 || echo "  使用 npm pack --dry-run 查看详情"

        echo ""
        echo "💡 环境检查完成！"
        echo ""
        echo "要发布占位包，请运行:"
        echo "  ./publish-placeholder.sh publish"
        echo ""
        ;;

    publish)
        echo "🚀 准备发布占位包..."
        echo ""

        # 确认
        echo "⚠️  警告: 这将发布占位包到 NPM"
        echo ""
        echo "包名: agentic-warden"
        echo "版本: $(node -p "require('./package.json').version")"
        echo ""
        read -p "确认发布？(y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "已取消"
            exit 0
        fi

        # 发布
        echo "  正在发布..."
        npm publish --access public

        if [ $? -eq 0 ]; then
            echo ""
            echo "✅ 占位包发布成功！"
            echo ""
            echo "用户现在运行 'npx agentic-warden' 将看到提示消息。"
            echo ""
        else
            echo ""
            echo "❌ 发布失败"
            exit 1
        fi
        ;;

    *)
        echo "❌ 未知命令: $COMMAND"
        echo ""
        echo "用法:"
        echo "  $0 check    - 检查环境"
        echo "  $0 publish  - 发布占位包"
        echo ""
        exit 1
        ;;
esac

echo ""
echo "=========================================="
echo "完成"
echo "=========================================="

# 恢复原始文件
echo ""
echo "📋 恢复原始文件..."
if [ -f "package.json.backup" ]; then
    mv package.json.backup package.json
    echo "  ✓ 已恢复 package.json"
fi

if [ -f ".npmignore.backup" ]; then
    mv .npmignore.backup .npmignore
else
    rm -f .npmignore
fi

echo ""
echo "✅ 清理完成"
echo ""
