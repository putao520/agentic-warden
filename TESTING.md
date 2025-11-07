# Agentic-Warden 测试体系指南

## 📋 概述

Agentic-Warden 项目采用多层次、全覆盖的测试体系，确保代码质量和功能可靠性。测试体系包括：

- **单元测试** - 测试独立的功能模块
- **集成测试** - 测试模块间的协作
- **CLI测试** - 测试命令行界面
- **TUI测试** - 测试终端用户界面
- **性能测试** - 测试系统性能
- **覆盖率分析** - 监控测试覆盖率

## 🏗️ 测试架构

```
测试体系架构
├── 单元测试 (src/*/tests.rs)
│   ├── Provider 模块测试
│   ├── Sync 模块测试
│   ├── TUI 模块测试
│   └── 核心模块测试
├── 集成测试 (tests/integration/)
│   ├── CLI 集成测试
│   ├── Provider 集成测试
│   ├── Sync 集成测试
│   └── TUI 集成测试
├── 测试工具 (src/test_utils/)
│   ├── Mock 对象
│   ├── 测试环境管理
│   ├── 测试断言工具
│   └── 外部依赖模拟
└── 自动化脚本 (scripts/)
    ├── 测试运行器
    ├── 验证脚本
    └── CI/CD 集成
```

## 🚀 快速开始

### 运行所有测试

```bash
# Linux/macOS
./scripts/test_runner.sh

# Windows
.\scripts\run_all_tests.ps1
```

### 运行特定类型的测试

```bash
# 单元测试
./scripts/test_runner.sh unit

# 集成测试
./scripts/test_runner.sh integration

# CLI测试
./scripts/test_runner.sh cli

# TUI测试
./scripts/test_runner.sh tui

# 性能测试
./scripts/test_runner.sh performance

# 覆盖率报告
./scripts/test_runner.sh coverage

# 快速测试
./scripts/test_runner.sh quick

# 冒烟测试
./scripts/test_runner.sh smoke
```

### 使用 Cargo 直接运行

```bash
# 运行所有测试
cargo test

# 运行特定模块的测试
cargo test --lib provider
cargo test --lib sync
cargo test integration

# 运行特定测试
cargo test test_provider_manager_creation
cargo test test_sync_push_operation

# 详细输出
cargo test -- --nocapture

# 并行运行
cargo test --release --jobs 8
```

## 🧪 测试编写指南

### 单元测试

单元测试应该放在对应模块的 `tests.rs` 文件中：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    async_test!(test_feature_name, {
        // 测试代码
        assert_eq!(result, expected);
        Ok(())
    });

    // 或者使用标准测试
    #[tokio::test]
    async fn test_another_feature() {
        let _env = TestEnvironment::setup().await?;
        // 测试代码
    }
}
```

### 集成测试

集成测试放在 `tests/integration/` 目录中：

```rust
use agentic_warden::test_utils::*;
use crate::integration::setup_integration_test;

#[tokio::test]
async fn test_integration_scenario() -> TestResult<()> {
    let _env = setup_integration_test().await?;

    // 测试代码
    Ok(())
}
```

### 测试工具使用

```rust
use crate::test_utils::*;

// 创建测试环境
let _env = TestEnvironment::setup().await?;

// 创建Mock对象
let mut mock_service = create_mock_google_drive_service();

// 创建测试数据
let config = TestFixtures::basic_provider_config();

// 使用断言
assert_file_exists(&file_path);
assert_provider_config(&provider, "expected_name");

// 设置环境变量
std::env::set_var("TEST_MODE", "1");
```

## 🎭 Mock 和外部依赖模拟

### Google Drive API Mock

```rust
let mut mock_service = create_mock_google_drive_service();
mock_service.expect_list_folder_files()
    .returning(|_| Ok(vec![]));
```

### OAuth Mock

```rust
let mut mock_oauth = create_mock_oauth_client();
mock_oauth.expect_exchange_code_for_token()
    .returning(|_, _| Ok(mock_token_response));
```

### 网络模拟

```rust
let mut mock_server = MockHttpServer::new().await?;
mock_server.setup_google_drive_api().await?;
```

## 📊 测试覆盖率

### 生成覆盖率报告

```bash
# 安装工具
cargo install cargo-llvm-cov

# 生成HTML报告
cargo llvm-cov --html --output-dir coverage

# 生成LCOV报告
cargo llvm-cov --lcov --output-path lcov.info

# 查看覆盖率
open coverage/index.html
```

### 覆盖率目标

- **整体覆盖率**: ≥ 90%
- **核心模块覆盖率**: ≥ 95%
- **新增功能覆盖率**: 100%

## 🔧 测试环境配置

### 环境变量

```bash
export AGENTIC_WARDEN_TEST_MODE=1      # 启用测试模式
export SKIP_NETWORK_CALLS=1            # 跳过网络调用
export RUST_LOG=debug                   # 设置日志级别
export RUST_BACKTRACE=1                 # 显示详细错误
```

### 测试配置文件

测试配置会自动创建在临时目录中，包括：

- `test-providers.json` - Provider测试配置
- `test-auth.json` - OAuth测试配置
- `test-sync.json` - 同步测试配置

## 🐳 Docker 测试

### 运行Docker测试

```bash
./scripts/test_runner.sh --docker all

# 或手动运行
docker build -t agentic-warden-test -f Dockerfile.test .
docker run --rm -v $(pwd):/workspace -w /workspace agentic-warden-test ./scripts/test_runner.sh all
```

### Dockerfile.test

```dockerfile
FROM rust:1.70

WORKDIR /workspace

# 安装测试依赖
RUN cargo install cargo-llvm-cov cargo-criterion

# 复制源代码
COPY . .

# 运行测试
CMD ["./scripts/test_runner.sh", "all"]
```

## 📈 性能测试

### 运行基准测试

```bash
# 安装工具
cargo install cargo-criterion

# 运行基准测试
cargo criterion

# 查看报告
open target/criterion/report/index.html
```

### 性能测试示例

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_function(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| {
            // 要测试的代码
            black_box(my_function(black_box(input_data)))
        })
    });
}

criterion_group!(benches, benchmark_function);
criterion_main!(benches);
```

## 🔍 测试验证

### 运行验证脚本

```bash
# Python验证脚本（需要Python 3.7+）
python3 scripts/validate_tests.py

# 检查测试质量
python3 scripts/validate_tests.py --project-root .
```

### 验证内容

- 测试覆盖率是否达标
- 测试命名规范
- 测试文档完整性
- Mock覆盖率
- 性能回归检测

## 🚨 CI/CD 集成

### GitHub Actions

项目配置了完整的CI/CD流水线：

- **代码质量检查**: Clippy, rustfmt, 文档生成
- **多平台测试**: Linux, Windows, macOS
- **自动化构建**: Release 模式构建
- **安全审计**: 依赖漏洞扫描
- **性能基准**: 回归检测
- **覆盖率报告**: 自动生成和上传

### 触发条件

- **Push**: `main`, `master`, `develop` 分支
- **Pull Request**: 针对 `main`, `master` 分支
- **定时任务**: 每天凌晨2点
- **标签推送**: 自动触发发布流程

## 🛠️ 故障排除

### 常见问题

#### 1. 测试超时

```bash
# 增加超时时间
./scripts/test_runner.sh -t 600 all
```

#### 2. 网络测试失败

```bash
# 跳过网络调用
export SKIP_NETWORK_CALLS=1
cargo test
```

#### 3. TUI测试失败

TUI测试在某些CI环境中可能因为终端问题失败，这是正常的。

#### 4. 依赖问题

```bash
# 清理并重新安装
cargo clean
cargo build
```

### 调试技巧

#### 1. 启用详细日志

```bash
RUST_LOG=debug cargo test -- --nocapture
```

#### 2. 运行单个测试

```bash
cargo test test_specific_function
```

#### 3. 使用GDB调试

```bash
cargo test --no-default-features --features debug
gdb target/debug/deps/test_name-*
```

## 📚 最佳实践

### 1. 测试命名

- 使用描述性的测试名称
- 采用 `should_` 前缀或描述性命名
- 包含测试的具体场景

### 2. 测试组织

- 按功能模块组织测试
- 使用测试工具和辅助函数
- 保持测试独立和可重复

### 3. Mock 使用

- 为外部依赖创建Mock
- 使用场景预设
- 验证Mock的覆盖率

### 4. 错误处理

- 测试成功路径和错误路径
- 验证错误信息的准确性
- 测试边界条件

### 5. 性能考虑

- 使用测试环境变量
- 避免真实的网络调用
- 合理设置测试超时

## 📝 贡献指南

### 添加新测试

1. 确定测试类型（单元/集成）
2. 选择合适的测试位置
3. 使用测试工具和Mock
4. 遵循命名和组织规范
5. 验证覆盖率贡献

### 维护测试

1. 定期检查测试覆盖率
2. 更新Mock以匹配API变化
3. 优化测试性能
4. 更新文档和示例

## 🤝 获取帮助

如果在测试过程中遇到问题：

1. 查看本文档和代码注释
2. 检查现有的测试示例
3. 使用调试技巧
4. 在项目仓库提交Issue

---

**最后更新**: 2025-11-06
**版本**: 1.0.0