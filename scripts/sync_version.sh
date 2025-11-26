#!/bin/bash

# 版本同步脚本
# 确保Cargo.toml版本与SPEC/VERSION保持一致

set -e

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 脚本目录
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${GREEN}Agentic-Warden 版本同步工具${NC}"
echo "========================================"

# 读取当前版本
cd "$PROJECT_ROOT"
SPEC_VERSION=$(cat SPEC/VERSION 2>/dev/null | tr -d '\n')
CARGO_VERSION=$(grep '^version' Cargo.toml 2>/dev/null | cut -d'"' -f2)

echo -e "SPEC/VERSION:\t${YELLOW}v$SPEC_VERSION${NC}"
echo -e "Cargo.toml:\t${YELLOW}v$CARGO_VERSION${NC}"

# 检查版本一致性
if [ "$SPEC_VERSION" = "$CARGO_VERSION" ]; then
    echo -e "\n${GREEN}✓ 版本号一致${NC}"
    exit 0
else
    echo -e "\n${RED}✗ 版本号不一致！${NC}"
    echo -e "${YELLOW}正在同步...${NC}"

    # 更新Cargo.toml
    sed -i "s/^version = \".*\"/version = \"$SPEC_VERSION\"/" Cargo.toml

    # 验证更新
    CARGO_VERSION=$(grep '^version' Cargo.toml | cut -d'"' -f2)

    if [ "$SPEC_VERSION" = "$CARGO_VERSION" ]; then
        echo -e "\n${GREEN}✓ 版本同步成功！${NC}"
        echo -e "新的版本号: v$SPEC_VERSION"
    else
        echo -e "\n${RED}✗ 版本同步失败！${NC}"
        exit 1
    fi
fi

# 可选：检查CHANGELOG
echo ""
read -p "是否检查CHANGELOG.md包含v$SPEC_VERSION的版本记录? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    if grep -q "## v$SPEC_VERSION" SPEC/05-CHANGELOG.md 2>/dev/null; then
        echo -e "${GREEN}✓ CHANGELOG.md已包含v$SPEC_VERSION${NC}"
    else
        echo -e "${YELLOW}⚠ CHANGELOG.md未找到v$SPEC_VERSION${NC}"
        echo "建议在CHANGELOG.md中添加版本记录"
    fi
fi

echo -e "\n${GREEN}版本同步完成！${NC}"
