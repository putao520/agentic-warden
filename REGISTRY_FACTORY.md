# Registry Factory - 双模式任务管理系统

## 概述

Agentic-Warden 实现了进程内双模式任务管理系统，通过工厂模式在同一进程中维护两个独立的任务注册表：

1. **CLI任务注册表** (`TaskRegistry`) - 跨进程共享内存
2. **MCP任务注册表** (`InProcessRegistry`) - 进程内独享

## 架构图

```text
┌───────────────────────────────────────────────────────────┐
│              RegistryFactory (Singleton)                  │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  ┌─────────────────────┐      ┌──────────────────────┐  │
│  │  TaskRegistry       │      │ InProcessRegistry    │  │
│  │  (SharedMemory)     │      │   (HashMap)          │  │
│  │                     │      │                      │  │
│  │  • 跨进程共享       │      │  • 进程内独享        │  │
│  │  • CLI任务          │      │  • MCP任务           │  │
│  │  • wait命令监控     │      │  • pwait命令监控     │  │
│  └─────────────────────┘      └──────────────────────┘  │
│           ↑                            ↑                  │
│           │                            │                  │
│    TaskSource::Cli            TaskSource::Mcp            │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

## 使用方式

### 1. 获取工厂实例（全局单例）

```rust
use agentic_warden::registry_factory::{RegistryFactory, TaskSource};

// 获取全局工厂实例
let factory = RegistryFactory::instance();
```

### 2. 获取CLI任务注册表

```rust
// 用于CLI启动的任务（跨进程共享）
let cli_registry = factory.get_cli_registry()?;

// 注册任务
cli_registry.register(pid, &task_record)?;

// 等待任务
// 使用: agentic-warden wait
```

### 3. 获取MCP任务注册表

```rust
// 用于MCP启动的任务（进程内独享）
let mcp_registry = factory.get_mcp_registry();

// 注册任务
mcp_registry.register(pid, &task_record)?;

// 等待任务
// 使用: agentic-warden pwait
```

### 4. 根据任务来源自动选择

```rust
use agentic_warden::registry_factory::{RegistryFactory, TaskSource, RegistryType};

let factory = RegistryFactory::instance();

// 根据任务来源获取对应的注册表
let registry = factory.get_registry_for(TaskSource::Mcp)?;

match registry {
    RegistryType::Cli(reg) => {
        // 使用TaskRegistry
        reg.register(pid, &task_record)?;
    }
    RegistryType::Mcp(reg) => {
        // 使用InProcessRegistry
        reg.register(pid, &task_record)?;
    }
}
```

## 关键特性

### 1. 单例模式
- 工厂实例全局唯一
- 每种类型的注册表在进程内只有一个实例

### 2. 懒加载
- CLI注册表延迟初始化（首次使用时连接共享内存）
- MCP注册表预先创建（进程启动时）

### 3. 线程安全
- 所有操作都是线程安全的
- 可以在多线程环境下安全使用

### 4. 隔离性
- CLI任务和MCP任务完全隔离
- 互不干扰，各自独立管理

## 实际应用场景

### 场景1：CLI命令启动任务

```rust
use agentic_warden::registry_factory::{RegistryFactory, TaskSource};

// 在CLI命令处理中
let factory = RegistryFactory::instance();
let registry = factory.get_cli_registry()?;

// 启动任务
supervisor::execute_cli(&registry, &cli_type, &args, provider).await?;

// 等待任务（另一个进程中）
// $ agentic-warden wait
```

### 场景2：MCP服务器启动任务

```rust
use agentic_warden::registry_factory::RegistryFactory;

// 在MCP服务器中
pub struct AgenticWardenMcpServer {
    provider_manager: Arc<Mutex<ProviderManager>>,
}

impl AgenticWardenMcpServer {
    pub fn registry(&self) -> Arc<InProcessRegistry> {
        // 从工厂获取全局MCP注册表
        RegistryFactory::instance().get_mcp_registry()
    }

    pub async fn start_task(&self, ...) {
        let registry = self.registry();
        supervisor::execute_cli(&*registry, &cli_type, &args, provider).await?;
    }
}
```

### 场景3：查看注册表状态

```rust
use agentic_warden::registry_factory::RegistryFactory;

let factory = RegistryFactory::instance();
let stats = factory.get_stats();

println!("CLI Registry initialized: {}", stats.cli_initialized);
println!("MCP Registry initialized: {}", stats.mcp_initialized);
```

## MCP + pwait 完整工作流程

### 架构说明

MCP服务器通过工厂模式使用 `InProcessRegistry` 来管理所有MCP启动的任务。这些任务与CLI启动的任务完全隔离，使用独立的存储和等待命令。

### 工作流程图

```text
┌─────────────────────────────────────────────────────────────┐
│                     Claude Code (MCP客户端)                  │
└────────────────────┬────────────────────────────────────────┘
                     │ MCP请求：启动多个并发任务
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              Agentic-Warden MCP Server                       │
│                                                              │
│  ┌──────────────────────────────────────────────┐           │
│  │  AgenticWardenMcpServer                      │           │
│  │                                              │           │
│  │  fn registry() -> Arc<McpRegistry> {         │           │
│  │      RegistryFactory::instance()             │───────┐   │
│  │          .get_mcp_registry()                 │       │   │
│  │  }                                           │       │   │
│  └──────────────────────────────────────────────┘       │   │
│                     │                                   │   │
│                     │ 逐个启动任务                      │   │
│                     ▼                                   │   │
│  ┌──────────────────────────────────────────────┐       │   │
│  │  supervisor::execute_cli(                    │       │   │
│  │      &registry,  // McpRegistry              │◄──────┘   │
│  │      &cli_type,                              │           │
│  │      &args,                                  │           │
│  │      provider                                │           │
│  │  )                                           │           │
│  └──────────────────────────────────────────────┘           │
│                     │                                       │
│                     │ 所有任务注册到                         │
│                     ▼                                       │
│  ┌──────────────────────────────────────────────┐           │
│  │      InProcessRegistry (HashMap)             │           │
│  │  ┌────────┐  ┌────────┐  ┌────────┐         │           │
│  │  │Task 1  │  │Task 2  │  │Task 3  │         │           │
│  │  └────────┘  └────────┘  └────────┘         │           │
│  └──────────────────────────────────────────────┘           │
│                                                              │
│  返回task JSON:                                              │
│  {                                                          │
│    "tool": "bash",                                         │
│    "command": "agentic-warden pwait --timeout 12h"         │
│  }                                                         │
└────────────────────┬─────────────────────────────────────────┘
                     │ 返回等待任务
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                   Claude Code (MCP客户端)                    │
│                                                              │
│  解析task JSON，使用Bash工具执行:                             │
│  $ agentic-warden pwait --timeout 12h                       │
└────────────────────┬─────────────────────────────────────────┘
                     │ 阻塞等待
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              pwait_mode::run()                               │
│                                                              │
│  1. 从工厂获取MCP注册表:                                      │
│     let registry = RegistryFactory::instance()              │
│                        .get_mcp_registry();                 │
│                                                              │
│  2. 轮询InProcessRegistry中的所有MCP任务                      │
│                                                              │
│  3. 等待所有任务完成                                          │
│                                                              │
│  4. 返回执行报告                                             │
└─────────────────────────────────────────────────────────────┘
```

### 核心代码示例

#### 1. MCP Server 获取注册表

```rust
// src/mcp.rs
use agentic_warden::registry_factory::{McpRegistry, RegistryFactory};

pub struct AgenticWardenMcpServer {
    provider_manager: Arc<Mutex<ProviderManager>>,
}

impl AgenticWardenMcpServer {
    pub fn registry(&self) -> Arc<McpRegistry> {
        // 从全局工厂获取MCP注册表（单例）
        RegistryFactory::instance().get_mcp_registry()
    }
}
```

#### 2. MCP 启动并发任务

```rust
// src/mcp.rs - start_concurrent_tasks方法
pub async fn start_concurrent_tasks(&self, tasks: Vec<TaskRequest>) -> Result<Value> {
    let registry = self.registry();  // 获取MCP注册表

    for task in tasks {
        // 使用supervisor启动每个任务（后台）
        tokio::spawn(async move {
            supervisor::execute_cli(
                &*registry,      // 使用MCP注册表
                &cli_type,
                &args,
                provider
            ).await
        });
    }

    // 返回pwait任务
    Ok(json!({
        "task": {
            "tool": "bash",
            "command": "agentic-warden pwait --timeout 12h",
            "timeout_ms": 43200000
        },
        "note": "Use 'pwait' to wait for MCP tasks (InProcessRegistry)"
    }))
}
```

#### 3. pwait 等待MCP任务

```rust
// src/pwait_mode.rs
use agentic_warden::registry_factory::RegistryFactory;

pub fn run() -> Result<WaitReport, PWaitError> {
    // 从工厂获取MCP注册表
    let registry = RegistryFactory::instance().get_mcp_registry();

    // 使用MCP注册表等待任务
    run_with_registry(&registry)
}

pub fn run_with_registry(registry: &McpRegistry) -> Result<WaitReport, PWaitError> {
    loop {
        // 从InProcessRegistry读取所有MCP任务
        let entries = registry.list_entries()?;

        if entries.is_empty() {
            break;  // 所有任务完成
        }

        // 检查任务状态，清理完成的任务
        thread::sleep(Duration::from_secs(1));
    }

    // 返回执行报告
    Ok(WaitReport { /* ... */ })
}
```

### 关键设计点

#### 1. 存储隔离

```rust
// CLI任务：跨进程共享内存
let cli_registry = RegistryFactory::instance().get_cli_registry()?;
// 存储位置：SharedMemoryStorage（操作系统共享内存）

// MCP任务：进程内HashMap
let mcp_registry = RegistryFactory::instance().get_mcp_registry();
// 存储位置：InProcessStorage（进程内HashMap）
```

**完全隔离**：
- CLI任务和MCP任务存储在不同的物理位置
- `wait` 命令读取 SharedMemoryStorage
- `pwait` 命令读取 InProcessStorage
- **不可混用**：使用 `wait` 无法看到MCP任务！

#### 2. 工厂单例保证

```rust
// 无论在代码的任何位置调用，都返回同一个实例
let registry1 = RegistryFactory::instance().get_mcp_registry();
let registry2 = RegistryFactory::instance().get_mcp_registry();

assert!(Arc::ptr_eq(&registry1, &registry2));  // ✅ 指向同一对象
```

#### 3. 命令对应关系

| 命令 | 读取存储 | 适用场景 |
|------|----------|----------|
| `agentic-warden wait` | SharedMemoryStorage | CLI启动的任务 |
| `agentic-warden pwait` | InProcessStorage | MCP启动的任务 |

⚠️ **重要警告**：
- 如果MCP返回 `wait` 命令，Claude Code执行后会发现"没有任务"（因为读取了错误的存储）
- 如果CLI任务使用 `pwait` 命令，也会发现"没有任务"
- 必须确保命令与任务来源匹配！

## 对比：两套系统的差异

| 特性 | CLI模式 | MCP模式 |
|------|---------|---------|
| **注册表类型** | `TaskRegistry` | `InProcessRegistry` |
| **存储方式** | 共享内存 | 进程内HashMap |
| **可见性** | 跨进程 | 仅本进程 |
| **等待命令** | `wait` | `pwait` |
| **初始化** | 懒加载 | 预加载 |
| **获取方式** | `get_cli_registry()` | `get_mcp_registry()` |
| **典型用途** | CLI命令行任务 | MCP Server任务 |

## 性能考虑

### CLI模式（SharedMemory）
- **优点**：跨进程可见，多个进程可以监控同一任务
- **缺点**：需要共享内存同步，性能稍低
- **适用场景**：需要跨进程监控的任务

### MCP模式（InProcess）
- **优点**：纯内存操作，性能更高
- **缺点**：只在当前进程可见
- **适用场景**：MCP服务器内部任务管理

## 测试

工厂模式包含完整的单元测试：

```rust
#[test]
fn test_singleton_instance() {
    let factory1 = RegistryFactory::instance();
    let factory2 = RegistryFactory::instance();
    assert!(std::ptr::eq(factory1, factory2));
}

#[test]
fn test_mcp_registry_always_available() {
    let factory = RegistryFactory::instance();
    let mcp1 = factory.get_mcp_registry();
    let mcp2 = factory.get_mcp_registry();
    assert!(Arc::ptr_eq(&mcp1, &mcp2));
}
```

## 未来扩展

工厂模式设计为可扩展，未来可以轻松添加更多类型的注册表：

```rust
pub enum TaskSource {
    Cli,
    Mcp,
    // 未来可添加：
    // Api,      // API服务器任务
    // Scheduler, // 定时任务
    // Custom,    // 自定义任务
}
```

## 总结

通过工厂模式，Agentic-Warden实现了：
- ✅ 进程内双模式任务管理
- ✅ 统一的接口，不同的实现
- ✅ 全局单例，确保唯一性
- ✅ 完全隔离，互不干扰
- ✅ 灵活扩展，易于维护
