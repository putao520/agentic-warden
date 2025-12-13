# Universal Release Manager - 通用发布版本管理器

**Skill ID**: release-manager
**Version**: 1.0.0
**Last Updated**: 2025-11-12

---

## 🎯 技能概述

通用发布版本管理器是一个自适应的版本发布工具，支持多编程语言的包管理器，解决手动版本管理的问题。

### 🚨 核心问题
- **版本号不一致**: Cargo.toml, package.json, pyproject.toml 等文件版本号不同步
- **手动更新痛苦**: 发布时需要手动更新多个配置文件
- **人为错误**: 容易遗漏某个文件或写错版本号
- **开发频率冲突**: 高频开发 vs 低频发布的节奏不匹配

### 💡 解决方案
- **自动检测**: 智能识别项目的包管理器类型
- **版本同步**: 一次性更新所有相关配置文件
- **发布流程**: 自动创建Git标签、推送、触发CI/CD
- **多语言支持**: 支持 Rust, Node.js, Python, Go, Java, Ruby, PHP 等

---

## 🛠️ 支持的包管理器

| 语言 | 包管理器 | 配置文件 | 检测命令 |
|------|----------|----------|----------|
| **Rust** | Cargo | `Cargo.toml` | `cargo --version` |
| **Node.js** | npm/yarn/pnpm | `package.json` | `npm --version` |
| **Python** | pip/poetry | `pyproject.toml`, `setup.py` | `pip --version` |
| **Go** | Go modules | `go.mod` | `go version` |
| **Java** | Maven/Gradle | `pom.xml`, `build.gradle` | `mvn --version` |
| **Ruby** | RubyGems | `Gemfile`, `*.gemspec` | `gem --version` |
| **PHP** | Composer | `composer.json` | `composer --version` |
| **Docker** | Docker | `Dockerfile`, `docker-compose.yml` | `docker --version` |

---

## 🎮 使用方式

### 方式1: Claude Code 技能调用
```bash
/release-manager --version 0.4.8 --release-notes "添加交互式AI CLI启动功能"
```

### 方式2: 命令行工具
```bash
./scripts/release.sh 0.4.8 "添加交互式AI CLI启动功能"
```

### 方式3: 交互式模式
```bash
./scripts/release.sh --interactive
```

---

## 🔧 核心功能

### 1. 项目检测 (Project Detection)
自动扫描当前目录，识别：
- 主要编程语言
- 包管理器类型
- 配置文件位置
- 版本号字段

### 2. 版本同步 (Version Synchronization)
一键更新所有相关文件：
- 主要配置文件 (Cargo.toml, package.json 等)
- API 文件中的版本引用
- README.md 中的版本说明
- CHANGELOG.md

### 3. Git 操作 (Git Operations)
自动执行 Git 操作：
- 创建版本标签
- 提交版本更新
- 推送到远程仓库
- 创建 GitHub Release

### 4. 发布触发 (Release Trigger)
自动触发相应的发布流程：
- GitHub Actions workflow
- CI/CD pipeline
- 包管理器发布

---

## 📋 实现架构

### 检测模块 (Detection Module)
```bash
detect_package_manager() {
    # 检测项目类型和包管理器
    if [[ -f "Cargo.toml" ]]; then echo "rust"; fi
    if [[ -f "package.json" ]]; then echo "nodejs"; fi
    if [[ -f "pyproject.toml" ]]; then echo "python"; fi
    # ... 更多检测逻辑
}
```

### 更新模块 (Update Module)
```bash
update_version() {
    local package_manager=$1
    local new_version=$2

    case $package_manager in
        "rust") update_cargo_toml $new_version ;;
        "nodejs") update_package_json $new_version ;;
        "python") update_pyproject_toml $new_version ;;
        # ... 更多更新逻辑
    esac
}
```

### 验证模块 (Validation Module)
```bash
validate_version() {
    # 验证版本号格式
    # 检查版本号是否递增
    # 验证文件完整性
}
```

---

## 🎯 使用场景

### 场景1: 日常发布
```bash
# 开发完成后，准备发布新版本
./scripts/release.sh 0.4.8 "修复关键bug，添加新功能"
```

### 场景2: 补丁版本
```bash
# 紧急修复
./scripts/release.sh 0.4.7.1 "修复安全漏洞"
```

### 场景3: 主版本升级
```bash
# 重大功能更新
./scripts/release.sh 1.0.0 "重新设计的架构，Breaking Changes"
```

---

## 🔍 高级功能

### 1. 自动生成 CHANGELOG
```bash
./scripts/release.sh 0.4.8 --auto-changelog
```

### 2. 预发布检查
```bash
./scripts/release.sh 0.4.8 --dry-run
```

### 3. 回滚功能
```bash
./scripts/release.sh --rollback 0.4.7
```

### 4. 多包项目支持
```bash
./scripts/release.sh 0.4.8 --monorepo
```

---

## 📊 配置文件

项目根目录创建 `.release-config.yml`:
```yaml
# Release configuration
project:
  name: "agentic-warden"
  type: "multi-language"  # single-language, multi-language, monorepo

package_managers:
  - type: "cargo"
    file: "Cargo.toml"
    version_field: "version"
  - type: "npm"
    file: "npm-package/package.json"
    version_field: "version"

git:
  auto_commit: true
  auto_tag: true
  auto_push: true
  tag_prefix: "v"

release:
  auto_changelog: true
  create_github_release: true
  trigger_ci: true

validation:
  check_version_format: true
  check_version_increment: true
  require_tests_pass: false
```

---

## 🚀 集成到 Claude Code

当用户说 "发布 v0.4.8" 或 "更新版本号" 时，自动触发此技能：

```python
def handle_release_request(version, notes=""):
    """处理发布请求"""
    # 1. 检测项目类型
    package_managers = detect_package_managers()

    # 2. 验证版本号
    if not validate_version(version):
        return "版本号格式错误，请使用 semantic versioning"

    # 3. 更新所有配置文件
    for pm in package_managers:
        update_version(pm, version)

    # 4. 创建 Git 提交和标签
    create_git_commit_and_tag(version, notes)

    # 5. 触发发布流程
    trigger_release_workflow(version)

    return f"✅ 版本 {version} 发布成功！"
```

---

## 📈 扩展性

### 添加新的包管理器支持
1. 在 `detect_package_manager()` 中添加检测逻辑
2. 实现 `update_<package_manager>_version()` 函数
3. 更新配置文件模板
4. 添加测试用例

### 插件系统
```bash
# 支持自定义插件
./scripts/release.sh 0.4.8 --plugin custom-deployment
```

---

## 🎉 总结

通用发布版本管理器解决了开发过程中的版本管理痛点：
- **自动化**: 减少手动操作，降低错误率
- **通用性**: 支持多语言多包管理器
- **灵活性**: 适应不同的发布节奏
- **集成性**: 无缝融入现有开发流程

这个技能将让版本发布变得像呼吸一样自然！