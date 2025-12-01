#!/bin/bash

# 版本更新脚本
# 用法: ./scripts/update-version.sh <version>
# 示例: ./scripts/update-version.sh 0.5.20

set -e

if [ -z "$1" ]; then
    echo "❌ 错误: 请指定版本号"
    echo "用法: $0 <version>"
    echo "示例: $0 0.5.20"
    exit 1
fi

NEW_VERSION=$1

# 检查版本格式 (简单验证)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "❌ 错误: 版本格式不正确，应该是 major.minor.patch 格式"
    echo "示例: 0.5.20"
    exit 1
fi

echo "🔄 开始更新版本到 $NEW_VERSION"

# 1. 更新 VERSION 文件
echo "📄 更新 VERSION 文件..."
sed -i "s/^.*$/$NEW_VERSION/" VERSION
echo "✅ VERSION 文件已更新: $NEW_VERSION"

# 2. 更新 Cargo.toml
echo "📦 更新 Cargo.toml..."
sed -i "s/^version = \"[^\"]*\"$/version = \"$NEW_VERSION\"/" Cargo.toml
echo "✅ Cargo.toml 已更新: $NEW_VERSION"

echo ""
echo "🎉 版本更新完成！"
echo ""
echo "📝 下一步操作:"
echo "1. 检查更新的文件: git diff"
echo "2. 提交变更: git add . && git commit -m \"feat: 版本更新到 $NEW_VERSION\""
echo "3. 创建 Git tag: git tag v$NEW_VERSION"
echo "4. 推送到远程: git push origin master && git push origin v$NEW_VERSION"
echo ""
echo "⚠️  注意: package.json 由 GitHub Actions 自动更新"