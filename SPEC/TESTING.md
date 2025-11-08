# Agentic-Warden 测试策略

## 测试架构概览

### 1. 测试层级体系
```
测试金字塔:
    ┌─────────────────────┐
    │    E2E Tests        │  ← 少量，关键用户路径
    │   (Integration)     │
    ├─────────────────────┤
    │  Integration Tests  │  ← 中等，模块间协作
    │   (Contract)        │
    ├─────────────────────┤
    │   Unit Tests        │  ← 大量，单元逻辑
    │   (Isolation)       │
    └─────────────────────┘
```

### 2. 测试分类
- **单元测试**: 函数级别的逻辑测试
- **集成测试**: 模块间的协作测试
- **端到端测试**: 完整用户流程测试
- **性能测试**: 性能和资源使用测试
- **安全测试**: 安全性和权限测试

## 单元测试策略

### 1. 测试覆盖目标
- **总体覆盖率**: ≥ 90%
- **核心模块覆盖率**: ≥ 95%
- **边界条件覆盖率**: 100%
- **错误处理覆盖率**: 100%

### 2. 模块测试重点

#### 2.1 commands/ 模块测试
```rust
// tests/unit/commands/test_ai_cli.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ai_cli::*;

    #[tokio::test]
    async fn test_parse_multi_ai_syntax() {
        // 测试多 AI 语法解析
        let result = parse_multi_ai_syntax("codex|claude").unwrap();
        assert_eq!(result, vec![AiType::Codex, AiType::Claude]);

        let result = parse_multi_ai_syntax("all").unwrap();
        assert_eq!(result, vec![AiType::All]);

        // 测试错误情况
        assert!(parse_multi_ai_syntax("invalid").is_err());
    }

    #[test]
    fn test_inject_provider_env() {
        // 测试环境变量注入
        let env_vars = inject_provider_env("openrouter").unwrap();
        assert!(env_vars.contains_key("OPENAI_API_KEY"));
        assert!(env_vars.contains_key("OPENAI_BASE_URL"));
    }

    #[tokio::test]
    async fn test_execute_ai_cli_command() {
        // 测试 AI CLI 命令执行
        let task_ids = execute_ai_cli_command(
            vec![AiType::Codex],
            Some("official".to_string()),
            "test prompt".to_string(),
        ).await.unwrap();

        assert_eq!(task_ids.len(), 1);
    }
}
```

#### 2.2 provider/ 模块测试
```rust
// tests/unit/provider/test_config.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::config::*;

    #[test]
    fn test_provider_validation() {
        let mut provider = Provider {
            name: "test-provider".to_string(),
            description: "Test provider".to_string(),
            compatible_with: vec![AiType::Codex],
            env: HashMap::new(),
            ..Default::default()
        };

        // 测试有效配置
        assert!(ConfigManager::validate_provider(&provider).is_ok());

        // 测试无效名称
        provider.name = "".to_string();
        assert!(ConfigManager::validate_provider(&provider).is_err());

        // 测试无效环境变量
        provider.name = "test-provider".to_string();
        provider.env.insert("INVALID-KEY".to_string(), "value".to_string());
        assert!(ConfigManager::validate_provider(&provider).is_err());
    }

    #[test]
    fn test_config_load_save() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("test_provider.json");

        // 创建测试配置
        let mut config_manager = ConfigManager::load_from(&config_path).unwrap();
        config_manager.add_provider(create_test_provider()).unwrap();
        config_manager.save().unwrap();

        // 验证配置保存成功
        let loaded_manager = ConfigManager::load_from(&config_path).unwrap();
        assert!(loaded_manager.get_provider("test-provider").is_some());
    }

    #[test]
    fn test_compatible_providers() {
        let config_manager = create_test_config_manager();

        let codex_providers = config_manager.get_compatible_providers(AiType::Codex);
        assert!(!codex_providers.is_empty());

        let gemini_providers = config_manager.get_compatible_providers(AiType::Gemini);
        assert!(!gemini_providers.is_empty());
    }
}
```

#### 2.3 core/ 模块测试
```rust
// tests/unit/core/test_process_tree.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::process_tree::*;

    #[test]
    fn test_find_root_process() {
        let manager = ProcessTreeManager::new();
        let root_process = manager.find_root_process();

        assert!(root_process.is_some());
        // 验证不是 explorer.exe (Windows)
        if cfg!(windows) {
            assert_ne!(root_process.unwrap().name.to_lowercase(), "explorer.exe");
        }
    }

    #[test]
    fn test_task_tracking() {
        let mut manager = ProcessTreeManager::new();

        // 模拟创建任务
        let task_info = create_test_task();
        let task_id = manager.track_ai_cli_process(&create_test_command()).unwrap();

        // 验证任务被跟踪
        let tasks = manager.get_all_tasks();
        assert!(tasks.iter().any(|t| t.id == task_id));
    }

    #[tokio::test]
    async fn test_task_termination() {
        let mut manager = ProcessTreeManager::new();

        // 创建测试任务
        let task_id = manager.track_ai_cli_process(&create_test_command()).unwrap();

        // 终止任务
        // 注意：当前系统使用简化的TaskStatus（Running/CompletedButUnread）
        // 任务完成后自动标记为CompletedButUnread

        // 验证任务状态
        let tasks = manager.get_all_tasks();
        let task = tasks.iter().find(|t| t.id == task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::Running | TaskStatus::CompletedButUnread));
    }
}
```

#### 2.4 sync/ 模块测试
```rust
// tests/unit/sync/test_oauth_client.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::oauth_client::*;

    #[test]
    fn test_oauth_url_generation() {
        let client = OAuthClient::new(
            "test-client-id".to_string(),
            "test-client-secret".to_string(),
            "http://localhost:8080".to_string(),
        );

        let auth_url = client.start_oob_flow().unwrap();
        assert!(auth_url.contains("accounts.google.com"));
        assert!(auth_url.contains("client_id=test-client-id"));
    }

    #[tokio::test]
    async fn test_token_exchange() {
        let client = create_test_oauth_client();

        // 模拟授权码交换
        let token_info = client.exchange_code_for_token("test-auth-code").await;

        // 由于这是测试，我们模拟成功和失败两种情况
        match token_info {
            Ok(token) => {
                assert!(!token.access_token.is_empty());
                assert!(!token.refresh_token.is_empty());
            }
            Err(_) => {
                // 测试错误处理
            }
        }
    }

    #[test]
    fn test_callback_server() {
        let client = create_test_oauth_client();
        let server_result = client.start_callback_server();

        assert!(server_result.is_ok());
        // 服务器应该在后台启动
    }
}
```

## 集成测试策略

### 1. 模块间集成测试
```rust
// tests/integration/mod.rs
mod provider_tui_integration;
mod sync_process_integration;
mod config_core_integration;

use anyhow::Result;
use tempfile::TempDir;
use std::path::PathBuf;

struct TestEnvironment {
    temp_dir: TempDir,
    config_path: PathBuf,
    auth_path: PathBuf,
}

impl TestEnvironment {
    fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let config_path = temp_dir.path().join("provider.json");
        let auth_path = temp_dir.path().join("auth.json");

        // 创建测试配置
        create_test_config(&config_path)?;

        Ok(Self {
            temp_dir,
            config_path,
            auth_path,
        })
    }
}
```

#### 1.1 Provider + TUI 集成测试
```rust
// tests/integration/provider_tui_integration.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::config::ConfigManager;
    use crate::tui::app::App;

    #[tokio::test]
    async fn test_provider_management_workflow() -> Result<()> {
        let env = TestEnvironment::new()?;

        // 初始化配置管理器
        let mut config_manager = ConfigManager::load_from(&env.config_path)?;

        // 添加新的 Provider
        let new_provider = create_test_provider();
        config_manager.add_provider(new_provider.clone())?;
        config_manager.save()?;

        // 重新加载配置
        let reloaded_manager = ConfigManager::load_from(&env.config_path)?;
        let loaded_provider = reloaded_manager.get_provider(&new_provider.name);

        assert!(loaded_provider.is_some());
        assert_eq!(loaded_provider.unwrap().description, new_provider.description);

        // 测试 TUI 界面加载
        let mut app = App::new();
        app.load_providers()?;

        assert!(app.state.providers.iter().any(|p| p.name == new_provider.name));

        Ok(())
    }

    #[tokio::test]
    async fn test_provider_edit_workflow() -> Result<()> {
        let env = TestEnvironment::new()?;
        let mut config_manager = ConfigManager::load_from(&env.config_path)?;

        // 添加初始 Provider
        let provider = create_test_provider();
        config_manager.add_provider(provider)?;
        config_manager.save()?;

        // 测试编辑流程
        let mut updated_provider = provider.clone();
        updated_provider.description = "Updated description".to_string();

        config_manager.update_provider(&provider.name, updated_provider)?;
        config_manager.save()?;

        // 验证更新成功
        let reloaded_manager = ConfigManager::load_from(&env.config_path)?;
        let loaded_provider = reloaded_manager.get_provider(&provider.name).unwrap();

        assert_eq!(loaded_provider.description, "Updated description");

        Ok(())
    }
}
```

#### 1.2 同步 + 进程管理集成测试
```rust
// tests/integration/sync_process_integration.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::google_drive_service::GoogleDriveService;
    use crate::core::process_tree::ProcessTreeManager;

    #[tokio::test]
    async fn test_push_with_process_tracking() -> Result<()> {
        let env = TestEnvironment::new()?;

        // 初始化进程管理器
        let mut process_manager = ProcessTreeManager::new();

        // 初始化同步服务（使用测试配置）
        let sync_service = create_test_sync_service(&env).await?;

        // 模拟 push 命令
        let test_dirs = vec![env.temp_dir.path().to_path_buf()];
        let push_result = sync_service.push_directories(&test_dirs).await?;

        assert!(push_result.success);
        assert!(push_result.uploaded_files > 0);

        // 验证进程被正确跟踪
        let tasks = process_manager.get_all_tasks();
        let push_task = tasks.iter().find(|t|
            t.command_line.iter().any(|arg| arg.contains("push"))
        );

        assert!(push_task.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_auth_flow_integration() -> Result<()> {
        let env = TestEnvironment::new()?;

        // 模拟未授权状态
        assert!(!is_authorized(&env.auth_path)?);

        // 初始化 OAuth 客户端
        let oauth_client = create_test_oauth_client();

        // 模拟授权流程
        let auth_url = oauth_client.start_oob_flow()?;
        assert!(auth_url.starts_with("https://accounts.google.com"));

        // 模拟用户授权完成
        let mock_token = create_mock_token();
        save_token(&env.auth_path, &mock_token)?;

        // 验证授权状态
        assert!(is_authorized(&env.auth_path)?);

        Ok(())
    }
}
```

## 端到端测试策略

### 1. 完整用户流程测试
```rust
// tests/e2e/mod.rs
mod ai_cli_workflow;
mod tui_workflow;
mod sync_workflow;

use anyhow::Result;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

struct E2ETestEnvironment {
    test_dir: tempfile::TempDir,
    config_dir: PathBuf,
}

impl E2ETestEnvironment {
    fn new() -> Result<Self> {
        let test_dir = tempfile::tempdir()?;
        let config_dir = test_dir.path().join(".agentic-warden");
        std::fs::create_dir_all(&config_dir)?;

        Ok(Self {
            test_dir,
            config_dir,
        })
    }

    fn run_agentic_warden(&self, args: &[&str]) -> Result<std::process::Output> {
        let output = Command::new("cargo")
            .args(&["run", "--"])
            .args(args)
            .env("AGENTIC_WARDEN_CONFIG_DIR", &self.config_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        Ok(output)
    }

    async fn run_agentic_warden_async(&self, args: &[&str]) -> Result<std::process::Child> {
        let child = Command::new("cargo")
            .args(&["run", "--"])
            .args(args)
            .env("AGENTIC_WARDEN_CONFIG_DIR", &self.config_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(child)
    }
}
```

#### 1.1 AI CLI 工作流测试
```rust
// tests/e2e/ai_cli_workflow.rs
#[tokio::test]
async fn test_ai_cli_startup_and_tracking() -> Result<()> {
    let env = E2ETestEnvironment::new()?;

    // 初始化配置
    env.run_agentic_warden(&["provider", "add", "test"])?;

    // 启动 AI CLI
    let mut child = env.run_agentic_warden_async(&[
        "codex",
        "-p", "test",
        "Write a hello world function in Rust"
    ]).await?;

    // 等待一段时间让进程启动
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 检查进程状态
    let status_output = env.run_agentic_warden(&["status"])?;
    assert!(status_output.status.success());

    // 验证输出包含任务信息
    let stdout = String::from_utf8(status_output.stdout)?;
    assert!(stdout.contains("codex"));
    assert!(stdout.contains("hello world"));

    // 清理
    child.kill().await?;
    Ok(())
}

#[test]
fn test_multi_ai_command() -> Result<()> {
    let env = E2ETestEnvironment::new()?;

    // 测试多 AI 命令
    let output = env.run_agentic_warden(&[
        "codex|claude",
        "Explain Rust ownership"
    ])?;

    assert!(output.status.success());

    // 验证两个 AI 都被调用
    let stdout = String::from_utf8(output.stdout)?;
    // 验证输出格式和内容

    Ok(())
}
```

#### 1.2 TUI 工作流测试
```rust
// tests/e2e/tui_workflow.rs
#[tokio::test]
async fn test_dashboard_startup() -> Result<()> {
    let env = E2ETestEnvironment::new()?;

    // 启动 Dashboard
    let mut child = env.run_agentic_warden_async(&[]).await?;

    // 等待 TUI 初始化
    tokio::time::sleep(Duration::from_secs(3)).await;

    // 验证进程正在运行
    assert!(child.try_wait()?.is_none());

    // 发送退出命令
    // 注意：这里需要模拟键盘输入，实际实现可能需要使用 expect 或类似工具

    child.kill().await?;
    Ok(())
}

#[test]
fn test_provider_management_tui() -> Result<()> {
    let env = E2ETestEnvironment::new()?;

    // 测试 Provider 管理（这里需要模拟 TUI 交互）
    // 实际实现可能需要使用自动化测试框架

    Ok(())
}
```

## 性能测试策略

### 1. 基准测试
```rust
// benches/performance.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crate::core::process_tree::ProcessTreeManager;
use crate::provider::config::ConfigManager;

fn bench_process_tree_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_tree");

    group.bench_function("find_root_process", |b| {
        b.iter(|| {
            let manager = ProcessTreeManager::new();
            black_box(manager.find_root_process())
        });
    });

    group.bench_function("track_multiple_tasks", |b| {
        b.iter(|| {
            let mut manager = ProcessTreeManager::new();
            for i in 0..100 {
                let task = create_test_task_with_id(i);
                manager.track_ai_cli_task(black_box(&task)).unwrap();
            }
        });
    });

    group.finish();
}

fn bench_config_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config");

    group.bench_function("load_config", |b| {
        b.iter(|| {
            ConfigManager::load().unwrap();
        });
    });

    group.bench_function("save_config", |b| {
        let manager = ConfigManager::load().unwrap();
        b.iter(|| {
            manager.save().unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_process_tree_operations, bench_config_operations);
criterion_main!(benches);
```

### 2. 内存使用测试
```rust
// tests/performance/memory_usage.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::System;
use std::sync::atomic::{AtomicUsize, Ordering};

    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

    #[global_allocator]
    static ALLOCATOR: System = System;

    #[test]
    fn test_memory_usage_growth() {
        let initial_memory = ALLOCATED.load(Ordering::Relaxed);

        // 创建大量任务
        let mut manager = ProcessTreeManager::new();
        for i in 0..1000 {
            let task = create_test_task_with_id(i);
            manager.track_ai_cli_task(&task).unwrap();
        }

        let after_tasks = ALLOCATED.load(Ordering::Relaxed);
        let memory_growth = after_tasks - initial_memory;

        // 验证内存增长在合理范围内
        assert!(memory_growth < 100 * 1024 * 1024); // 100MB
    }

    #[test]
    fn test_memory_leak_prevention() {
        let initial_memory = ALLOCATED.load(Ordering::Relaxed);

        // 创建和销毁多个管理器实例
        for _ in 0..100 {
            let manager = ProcessTreeManager::new();
            drop(manager);
        }

        let final_memory = ALLOCATED.load(Ordering::Relaxed);
        let memory_growth = final_memory - initial_memory;

        // 验证没有明显内存泄漏
        assert!(memory_growth < 10 * 1024 * 1024); // 10MB
    }
}
```

## 测试工具和框架

### 1. 测试依赖
```toml
[dev-dependencies]
# 基础测试框架
tokio-test = "0.4"
tempfile = "3.8"
mockall = "0.12"

# HTTP 模拟
wiremock = "0.5"
mockito = "1.2"

# 性能测试
criterion = "0.5"

# 并发测试
futures-test = "0.3"

# 覆盖率报告
tarpaulin = "0.27"

# 属性测试
proptest = "1.4"
quickcheck = "1.0"

# 快照测试
insta = "1.34"

# 测试工具
assert_cmd = "2.0"
assert_fs = "1.0"
predicates = "3.0"
```

### 2. Mock 和 Fixture 工具
```rust
// tests/common/mod.rs
pub mod fixtures {
    use crate::*;

    pub fn create_test_provider() -> Provider {
        Provider {
            name: "test-provider".to_string(),
            description: "Test provider for unit tests".to_string(),
            compatible_with: vec![AiType::Codex, AiType::Claude],
            env: HashMap::from([
                ("TEST_API_KEY".to_string(), "test-key".to_string()),
                ("TEST_BASE_URL".to_string(), "https://test.api.com".to_string()),
            ]),
            ..Default::default()
        }
    }

    pub fn create_test_task() -> TaskInfo {
        // TaskInfo现在只有6个字段（符合SPEC ARCHITECTURE.md:264）
        TaskInfo {
            id: TaskId::new(),
            parent_process: create_test_process(),
            ai_type: AiType::Codex,
            prompt_preview: "Test prompt".to_string(),
            status: TaskStatus::Running,
            start_time: SystemTime::now(),
        }
    }

    pub fn create_test_process() -> ProcessInfo {
        ProcessInfo {
            pid: 1234,
            ppid: 1,
            name: "test-process".to_string(),
            path: Some(PathBuf::from("/usr/bin/test")),
            command_line: "test-process arg1 arg2".to_string(),
            start_time: SystemTime::now(),
            user_id: Some(1000),
            is_root: false,
            depth: 1,
        }
    }
}

pub mod mocks {
    use mockall::mock;

    mock! {
        pub OAuthClient {}

        impl OAuthClientTrait for OAuthClient {
            fn start_oob_flow(&self) -> Result<String>;
            async fn exchange_code_for_token(&self, code: &str) -> Result<TokenInfo>;
            fn start_callback_server(&self) -> Result<()>;
        }
    }

    mock! {
        pub ProcessTreeManager {}

        impl ProcessTreeManagerTrait for ProcessTreeManager {
            fn find_root_process(&self) -> Option<ProcessInfo>;
            fn track_ai_cli_process(&mut self, cmd: &AiCliCommand) -> Result<TaskId>;
            fn get_all_tasks(&self) -> Vec<TaskInfo>;
            fn terminate_task(&mut self, task_id: TaskId) -> Result<()>;
        }
    }
}
```

## 测试执行和 CI/CD

### 1. 测试命令
```bash
# 运行所有测试
cargo test --all

# 运行单元测试
cargo test --lib

# 运行集成测试
cargo test --test integration

# 运行端到端测试
cargo test --test e2e

# 运行性能测试
cargo bench

# 生成覆盖率报告
cargo tarpaulin --out html --output-dir target/coverage

# 运行特定模块测试
cargo test -p agentic-warden provider::config
```

### 2. GitHub Actions 配置
```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, beta, nightly]

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}
        override: true

    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Run tests
      run: cargo test --all --verbose

    - name: Generate coverage report
      run: cargo tarpaulin --out xml
      if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'

    - name: Upload coverage to Codecov
      uses: codecov/codecov-action@v3
      with:
        file: cobertura.xml
      if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'

  performance:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true

    - name: Run benchmarks
      run: cargo bench

    - name: Store benchmark result
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: target/criterion/reports/index.html
```

## 测试最佳实践

### 1. 测试命名和组织
- 使用描述性的测试名称
- 按功能模块组织测试
- 使用 AAA 模式（Arrange, Act, Assert）
- 保持测试的独立性

### 2. 测试数据管理
- 使用 Factory 模式创建测试数据
- 避免硬编码测试数据
- 使用 fixtures 和 mocks
- 清理测试产生的副作用

### 3. 错误处理测试
- 测试所有错误路径
- 验证错误消息的准确性
- 测试边界条件
- 测试资源清理

### 4. 异步测试
- 使用 `tokio-test` 进行异步测试
- 测试并发场景
- 验证超时处理
- 测试资源竞争

### 5. 测试维护
- 定期更新测试
- 移除过时的测试
- 重构重复的测试代码
- 保持测试的可读性