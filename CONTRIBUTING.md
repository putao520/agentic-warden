# 贡献指南

感谢您对 agentic-warden 项目的关注！我们欢迎所有形式的贡献，包括但不限于：

- 🐛 Bug 报告
- ✨ 新功能建议
- 📝 文档改进
- 🔧 代码贡献
- 🧪 测试用例

## 🚀 如何贡献

### 1. 报告 Bug

如果您发现了 bug，请在 [Issues](https://github.com/putao520/agentic-warden/issues) 页面创建一个新的 issue，并包含以下信息：

- 📋 Bug 描述
- 🔄 重现步骤
- 🎯 期望行为
- 📸 实际行为截图（如适用）
- 💻 环境信息（操作系统、Rust 版本等）

### 2. 提出新功能建议

如果您有新功能的想法，请在 [Issues](https://github.com/putao520/agentic-warden/issues) 页面创建一个新的 issue，并详细描述：

- 💡 功能描述
- 🎯 使用场景
- 🔧 实现建议（可选）

### 3. 代码贡献

#### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden

# 安装 Rust（如果尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 构建项目
cargo build

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy -- -D warnings
```

#### 开发流程

1. **Fork 仓库**
   - 在 GitHub 页面点击 "Fork" 按钮

2. **创建功能分支**
   ```bash
   git checkout -b feature/amazing-feature
   ```

3. **进行开发**
   - 编写代码
   - 添加测试（如果适用）
   - 确保所有测试通过：`cargo test`
   - 运行代码检查：`cargo clippy`
   - 格式化代码：`cargo fmt`

4. **提交更改**
   ```bash
   git add .
   git commit -m "feat: add amazing feature"
   ```

5. **推送到分支**
   ```bash
   git push origin feature/amazing-feature
   ```

6. **创建 Pull Request**
   - 在 GitHub 页面创建 Pull Request
   - 填写详细的 PR 描述
   - 等待代码审查

## 📝 代码规范

### Rust 代码风格

- 使用 `cargo fmt` 进行代码格式化
- 使用 `cargo clippy` 进行代码检查
- 遵循 Rust 官方命名约定
- 为公共函数和模块编写文档注释

### 提交信息规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**类型 (type)**：
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式化（不影响功能）
- `refactor`: 代码重构
- `test`: 测试相关
- `chore`: 构建过程或辅助工具的变动

**示例**：
```
feat(supervisor): add multi-agent parallel execution

- Implement parallel task distribution across multiple AI agents
- Add shared memory synchronization for task tracking
- Improve error handling for agent failures

Closes #42
```

## 🧪 测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test process_tree

# 运行集成测试
cargo test --test integration

# 显示测试输出
cargo test -- --nocapture
```

### 编写测试

- 为新功能编写单元测试
- 添加集成测试来验证跨功能交互
- 使用 `#[cfg(test)]` 属性标记测试模块
- 确保测试具有描述性的名称

## 📖 文档

### 代码文档

- 为所有公共 API 编写文档注释
- 使用 `///` 进行行级文档
- 使用 `//!` 进行模块级文档

```rust
/// 在指定目录中执行 AI Agent 任务
/// 
/// # 参数
/// 
/// * `agent_type` - AI Agent 类型 (claude, codex, gemini)
/// * `task` - 要执行的任务描述
/// * `working_dir` - 工作目录路径
/// 
/// # 示例
/// 
/// ```rust
/// let result = execute_agent("claude", "写一个排序算法", "/tmp")?;
/// ```
pub fn execute_agent(agent_type: &str, task: &str, working_dir: &str) -> Result<()> {
    // 实现代码
}
```

### README 更新

- 添加新功能时更新 README
- 保持示例代码的准确性
- 更新安装和使用说明

## 🏷️ 版本发布

项目遵循 [Semantic Versioning](https://semver.org/) 规范：

- **主版本号 (MAJOR)**：不兼容的 API 修改
- **次版本号 (MINOR)**：向下兼容的功能性新增
- **修订号 (PATCH)**：向下兼容的问题修正

## 🤝 社区准则

### 行为准则

我们致力于为每个人提供友好、安全和欢迎的环境。请遵循以下准则：

- 🤗 **尊重他人** - 尊重不同的观点和经验
- 🚀 **建设性反馈** - 提供建设性的、有帮助的反馈
- 🎯 **专注主题** - 保持讨论与项目相关
- 🌍 **包容性** - 欢迎所有背景的贡献者
- 📚 **学习心态** - 对学习新事物持开放态度

### 沟通渠道

- **GitHub Issues**: Bug 报告和功能请求
- **GitHub Discussions**: 一般讨论和问答
- **Pull Requests**: 代码审查和技术讨论

## 🏆 贡献者认可

所有贡献者都会在项目中得到认可。感谢以下类型的贡献：

- 💻 代码贡献
- 📖 文档编写
- 🐛 Bug 报告
- 💡 功能建议
- 🧪 测试编写
- 🎨 设计改进
- 🌍 本地化翻译

## 📞 获取帮助

如果您在贡献过程中遇到任何问题，请随时：

1. 查看 [FAQ](https://github.com/putao520/agentic-warden/discussions/categories/q-a)
2. 在 [Discussions](https://github.com/putao520/agentic-warden/discussions) 中提问
3. 创建一个 Issue
4. 联系维护者

---

再次感谢您的贡献！🎉

如果您有任何问题或建议，请随时联系我们。