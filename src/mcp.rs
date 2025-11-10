//! MCP (Model Context Protocol) 服务器模块 - 基于rmcp库的标准实现
//!
//! 提供Agentic-Warden的MCP服务，允许外部AI CLI通过MCP协议调用功能
//!
//! # 实现方式
//!
//! 使用rmcp库提供标准的MCP v1.0实现：
//! - 使用 `#[tool_router]` 宏生成工具路由
//! - 使用 `#[tool]` 宏定义工具函数
//! - 使用 `#[tool_handler]` 宏实现ServerHandler
//! - 使用 `.serve(transport)` 启动stdio传输
//!
//! # 任务管理
//!
//! MCP服务器使用工厂模式获取进程内任务注册表：
//! - 通过 `RegistryFactory::instance().get_mcp_registry()` 获取全局唯一的MCP注册表
//! - 所有MCP启动的任务都存储在这个注册表中
//! - 与CLI任务注册表完全隔离

use rmcp::{
    tool, tool_router, tool_handler,
    handler::server::tool::ToolRouter,
    ServerHandler,
    ServiceExt,
};
use std::sync::Arc;
use std::future::Future;
use tokio::sync::Mutex;
use serde_json::Value;

use agentic_warden::provider::manager::ProviderManager;
use agentic_warden::registry_factory::{McpRegistry, RegistryFactory};

/// Agentic-Warden MCP服务器
///
/// 使用rmcp库实现标准的MCP v1.0服务器，提供以下工具：
/// - `monitor_processes`: 监控AI CLI进程
/// - `get_process_tree`: 获取进程树信息
/// - `get_provider_status`: 获取Provider配置
/// - `terminate_process`: 安全终止进程
/// - `start_concurrent_tasks`: 并发启动多个任务
/// - `get_task_command`: 获取单个任务的启动命令
#[derive(Clone)]
pub struct AgenticWardenMcpServer {
    /// Provider管理器
    provider_manager: Arc<Mutex<ProviderManager>>,
    /// 工具路由器（由rmcp宏生成）
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AgenticWardenMcpServer {
    /// 创建新的MCP服务器实例
    pub fn new(provider_manager: ProviderManager) -> Self {
        Self {
            provider_manager: Arc::new(Mutex::new(provider_manager)),
            tool_router: Self::tool_router(),
        }
    }

    /// 获取MCP任务注册表（从工厂获取全局单例）
    pub fn registry(&self) -> Arc<McpRegistry> {
        RegistryFactory::instance().get_mcp_registry()
    }

    /// 启动MCP服务器 (使用rmcp的标准stdio传输)
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🚀 Starting Agentic-Warden MCP Server");
        eprintln!("   Protocol: MCP v1.0 (rmcp-based)");
        eprintln!("   Transport: stdio");
        eprintln!("   Tools: 6 available");
        eprintln!();
        eprintln!("✓ Server ready. Use Ctrl+C to stop.");
        eprintln!();

        // 使用rmcp的标准stdio服务
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        self.serve(transport).await?.waiting().await?;

        eprintln!("✓ MCP server stopped gracefully");
        Ok(())
    }

    /// 监控所有运行的AI CLI进程及其状态
    ///
    /// 列出系统中所有进程，支持按名称过滤和AI CLI类型过滤。
    /// 对于AI CLI进程，自动识别其类型（claude/codex/gemini）。
    ///
    /// # 参数
    /// - `filter`: 可选的进程名称过滤字符串（不区分大小写）
    /// - `ai_only`: 是否只显示AI CLI进程
    ///
    /// # 返回
    /// 包含进程列表的JSON对象，每个进程包含：
    /// - pid: 进程ID
    /// - name: 进程名称
    /// - status: 进程状态
    /// - command: 完整命令行
    /// - parent_pid: 父进程ID
    /// - ai_cli_type: AI CLI类型（如果是AI CLI进程）
    #[tool(description = "Monitor AI CLI processes with optional filtering")]
    pub async fn monitor_processes(&self, filter: Option<String>, ai_only: Option<bool>) -> String {
        match get_system_processes().await {
            Ok(processes) => {
                let mut filtered_processes = Vec::new();

                for proc in processes {
                    // 如果设置了 ai_only，只保留 AI CLI 进程
                    if ai_only.unwrap_or(false) && proc.ai_cli_type.is_none() {
                        continue;
                    }

                    // 如果设置了 filter，只保留名称包含过滤字符串的进程
                    if let Some(ref f) = filter {
                        if !proc.name.to_lowercase().contains(&f.to_lowercase()) &&
                           !proc.command.to_lowercase().contains(&f.to_lowercase()) {
                            continue;
                        }
                    }

                    filtered_processes.push(serde_json::json!({
                        "pid": proc.pid,
                        "name": proc.name,
                        "status": proc.status,
                        "command": proc.command,
                        "parent_pid": proc.parent_pid,
                        "ai_cli_type": proc.ai_cli_type,
                    }));
                }

                serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "processes": filtered_processes,
                    "count": filtered_processes.len()
                })).unwrap_or_else(|e| format!("{{\"success\": false, \"error\": \"{}\"}}", e))
            },
            Err(e) => {
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": format!("Failed to list processes: {}", e)
                })).unwrap()
            }
        }
    }

    /// 获取指定进程的进程树信息
    ///
    /// 返回指定进程及其所有子孙进程的树状结构。
    ///
    /// # 参数
    /// - `pid`: 要查询的根进程ID
    ///
    /// # 返回
    /// 包含进程树的JSON对象：
    /// - root: 根进程信息
    /// - children: 子进程列表（递归）
    #[tool(description = "Get process tree information for a specific process")]
    pub async fn get_process_tree(&self, pid: u32) -> String {
        match get_system_processes().await {
            Ok(processes) => {
                fn build_tree(pid: u32, all_processes: &[ProcessInfo]) -> Option<Value> {
                    let proc = all_processes.iter().find(|p| p.pid == pid)?;
                    let children: Vec<Value> = all_processes
                        .iter()
                        .filter(|p| p.parent_pid == Some(pid))
                        .filter_map(|child| build_tree(child.pid, all_processes))
                        .collect();

                    Some(serde_json::json!({
                        "pid": proc.pid,
                        "name": proc.name,
                        "status": proc.status,
                        "command": proc.command,
                        "ai_cli_type": proc.ai_cli_type,
                        "children": children
                    }))
                }

                if let Some(tree) = build_tree(pid, &processes) {
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "tree": tree
                    })).unwrap()
                } else {
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": false,
                        "error": format!("Process {} not found", pid)
                    })).unwrap()
                }
            },
            Err(e) => {
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": format!("Failed to get process tree: {}", e)
                })).unwrap()
            }
        }
    }

    /// 获取Provider状态和配置
    ///
    /// 返回指定Provider的配置信息和可用性状态。
    ///
    /// # 参数
    /// - `provider_name`: 可选的Provider名称，如果不指定则返回所有Provider
    ///
    /// # 返回
    /// 包含Provider状态的JSON对象：
    /// - name: Provider名称
    /// - description: 描述
    /// - env_vars: 环境变量配置
    /// - compatible_with: 兼容的AI CLI类型列表
    #[tool(description = "Get provider configuration and status")]
    pub async fn get_provider_status(&self, provider_name: Option<String>) -> String {
        let pm = self.provider_manager.lock().await;

        if let Some(name) = provider_name {
            match pm.get_provider(&name) {
                Ok(provider) => {
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "provider": {
                            "name": provider.name,
                            "description": provider.description,
                            "env_vars": provider.env,
                            "compatible_with": provider.compatible_with,
                        }
                    })).unwrap()
                },
                Err(e) => {
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": false,
                        "error": format!("Provider '{}' not found: {}", name, e)
                    })).unwrap()
                }
            }
        } else {
            // 返回所有provider
            let all_providers = pm.list_providers();
            let providers_json: Vec<Value> = all_providers.iter().map(|(name, p)| {
                serde_json::json!({
                    "name": name,
                    "description": p.description,
                    "env_vars": p.env,
                    "compatible_with": p.compatible_with,
                })
            }).collect();

            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "providers": providers_json,
                "count": providers_json.len()
            })).unwrap()
        }
    }

    /// 安全终止指定进程
    ///
    /// 使用SIGTERM（优雅终止）或SIGKILL（强制终止）信号终止进程。
    ///
    /// # 参数
    /// - `pid`: 要终止的进程ID
    /// - `force`: 是否强制终止（SIGKILL）
    ///
    /// # 返回
    /// 包含操作结果的JSON对象
    #[tool(description = "Terminate a process safely")]
    pub async fn terminate_process(&self, pid: u32, force: Option<bool>) -> String {
        #[cfg(target_family = "unix")]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            let signal = if force.unwrap_or(false) {
                Signal::SIGKILL
            } else {
                Signal::SIGTERM
            };

            match kill(Pid::from_raw(pid as i32), signal) {
                Ok(_) => {
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "message": format!("Process {} terminated with {:?}", pid, signal),
                        "pid": pid,
                        "signal": format!("{:?}", signal)
                    })).unwrap()
                },
                Err(e) => {
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": false,
                        "error": format!("Failed to terminate process {}: {}", pid, e)
                    })).unwrap()
                }
            }
        }

        #[cfg(target_family = "windows")]
        {
            serde_json::to_string_pretty(&serde_json::json!({
                "success": false,
                "error": "Process termination not yet implemented on Windows"
            })).unwrap()
        }
    }

    /// 生成多个AI CLI任务的启动命令
    ///
    /// 为多个AI CLI任务生成可执行的bash命令。
    /// MCP客户端（如Claude Code）可以使用Bash工具并行执行这些命令。
    ///
    /// # 参数
    /// - `tasks`: 任务数组的JSON字符串，每个任务包含：
    ///   - ai_type: AI类型（claude/codex/gemini）
    ///   - task: 任务描述
    ///   - provider: 可选的Provider名称
    ///
    /// # 返回
    /// 包含所有任务命令的JSON对象：
    /// - success: 操作是否成功
    /// - tasks: 任务列表，包含每个任务的:
    ///   - ai_type: AI类型
    ///   - task: 任务描述
    ///   - provider: Provider名称（如果有）
    ///   - command: 完整的bash命令
    /// - count: 任务数量
    /// - message: 使用说明
    #[tool(description = "Generate commands for multiple AI CLI tasks to run concurrently")]
    pub async fn start_concurrent_tasks(&self, tasks_json: String) -> String {
        // 解析tasks数组
        let tasks: Vec<Value> = match serde_json::from_str(&tasks_json) {
            Ok(v) => v,
            Err(e) => {
                return serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": format!("Invalid JSON format: {}", e)
                })).unwrap();
            }
        };

        let mut results = Vec::new();

        for task_spec in tasks {
            let ai_type = match task_spec.get("ai_type").and_then(|v| v.as_str()) {
                Some(t) => t,
                None => {
                    results.push(serde_json::json!({
                        "success": false,
                        "error": "Missing 'ai_type' field"
                    }));
                    continue;
                }
            };

            let task = match task_spec.get("task").and_then(|v| v.as_str()) {
                Some(t) => t,
                None => {
                    results.push(serde_json::json!({
                        "success": false,
                        "error": "Missing 'task' field"
                    }));
                    continue;
                }
            };

            let provider = task_spec.get("provider").and_then(|v| v.as_str());

            // 验证AI类型
            if let Err(e) = parse_ai_type(ai_type) {
                results.push(serde_json::json!({
                    "success": false,
                    "ai_type": ai_type,
                    "error": format!("{}", e)
                }));
                continue;
            }

            // 构建启动命令
            let command = if let Some(p) = provider {
                format!("agent {} -p {} '{}'", ai_type, p, task.replace("'", "'\\''"))
            } else {
                format!("agent {} '{}'", ai_type, task.replace("'", "'\\''"))
            };

            results.push(serde_json::json!({
                "success": true,
                "ai_type": ai_type,
                "task": task,
                "provider": provider,
                "command": command
            }));
        }

        let success_count = results.iter().filter(|r| r.get("success") == Some(&serde_json::json!(true))).count();

        serde_json::to_string_pretty(&serde_json::json!({
            "success": true,
            "tasks": results,
            "count": success_count,
            "message": "Execute these commands using Bash tool with background mode (run_in_background: true) for concurrent execution"
        })).unwrap()
    }

    /// 获取单个AI CLI任务的启动命令
    ///
    /// 返回可以直接在MCP客户端（如Claude Code）中执行的bash命令。
    ///
    /// # 参数
    /// - `ai_type`: AI CLI类型（claude, codex, gemini）
    /// - `task`: 要执行的任务
    /// - `provider`: 可选的Provider名称
    ///
    /// # 返回
    /// 包含任务启动命令的JSON对象：
    /// - success: 操作是否成功
    /// - task: 任务对象，包含：
    ///   - description: 任务描述
    ///   - tool: "bash"
    ///   - command: 完整的shell命令
    ///   - timeout_ms: 超时时间（12小时）
    /// - ai_type: AI CLI类型
    /// - provider: Provider名称（如果有）
    /// - message: 使用说明
    #[tool(description = "Get command to start a single AI CLI task")]
    pub async fn get_task_command(&self, ai_type: String, task: String, provider: Option<String>) -> String {
        // 验证AI类型
        if let Err(e) = parse_ai_type(&ai_type) {
            return serde_json::to_string_pretty(&serde_json::json!({
                "success": false,
                "error": format!("{}", e)
            })).unwrap();
        }

        // 构建命令字符串
        let command = if let Some(ref p) = provider {
            format!("agent {} -p {} '{}'", ai_type, p, task.replace("'", "'\\''"))
        } else {
            format!("agent {} '{}'", ai_type, task.replace("'", "'\\''"))
        };

        // 构建任务描述
        let description = if let Some(ref p) = provider {
            format!("Execute {} task with provider {}: {}", ai_type, p, task)
        } else {
            format!("Execute {} task: {}", ai_type, task)
        };

        serde_json::to_string_pretty(&serde_json::json!({
            "success": true,
            "task": {
                "description": description,
                "tool": "bash",
                "command": command,
                "timeout_ms": 43200000
            },
            "ai_type": ai_type,
            "provider": provider,
            "message": "Execute the 'task' using Bash tool with 12h timeout"
        })).unwrap()
    }
}

// 使用tool_handler宏实现ServerHandler trait
#[tool_handler(router = self.tool_router)]
impl ServerHandler for AgenticWardenMcpServer {}

// ==================== 辅助函数和类型定义 ====================

/// 进程信息结构体
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub status: String,
    pub command: String,
    pub parent_pid: Option<u32>,
    pub ai_cli_type: Option<String>,
}

/// 获取系统进程列表
async fn get_system_processes() -> Result<Vec<ProcessInfo>, anyhow::Error> {
    let mut processes = Vec::new();

    #[cfg(target_family = "unix")]
    {
        for proc in psutil::process::processes()? {
            if let Ok(proc) = proc {
                let pid = proc.pid();
                let name = proc.name().unwrap_or_else(|_| String::from("unknown"));
                let command = match proc.cmdline() {
                    Ok(Some(args)) => args,
                    _ => name.clone(),
                };
                let status = proc.status()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|_| "unknown".to_string());
                let parent_pid = proc.ppid().ok().flatten();
                let ai_cli_type = if is_ai_cli_process(&name) || is_ai_cli_process(&command) {
                    Some(detect_ai_cli_type(&name))
                } else {
                    None
                };

                processes.push(ProcessInfo {
                    pid,
                    name,
                    status,
                    command,
                    parent_pid,
                    ai_cli_type,
                });
            }
        }
    }

    #[cfg(target_family = "windows")]
    {
        use sysinfo::{System, SystemExt, ProcessExt};
        let mut sys = System::new_all();
        sys.refresh_all();

        for (pid, proc) in sys.processes() {
            let pid = pid.as_u32();
            let name = proc.name().to_string();
            let command = proc.cmd().join(" ");
            let status = format!("{:?}", proc.status());
            let parent_pid = proc.parent().map(|p| p.as_u32());
            let ai_cli_type = if is_ai_cli_process(&name) || is_ai_cli_process(&command) {
                Some(detect_ai_cli_type(&name))
            } else {
                None
            };

            processes.push(ProcessInfo {
                pid,
                name,
                status,
                command,
                parent_pid,
                ai_cli_type,
            });
        }
    }

    Ok(processes)
}

/// 检测进程是否为AI CLI进程
fn is_ai_cli_process(name_or_cmd: &str) -> bool {
    let lower = name_or_cmd.to_lowercase();
    lower.contains("claude") ||
    lower.contains("codex") ||
    lower.contains("gemini") ||
    lower.contains("aichat") ||
    lower.contains("gpt")
}

/// 检测AI CLI类型
fn detect_ai_cli_type(name_or_cmd: &str) -> String {
    let lower = name_or_cmd.to_lowercase();
    if lower.contains("claude") {
        "claude".to_string()
    } else if lower.contains("codex") {
        "codex".to_string()
    } else if lower.contains("gemini") {
        "gemini".to_string()
    } else {
        "unknown".to_string()
    }
}

/// 解析AI类型字符串
fn parse_ai_type(ai_type: &str) -> Result<(), String> {
    match ai_type.to_lowercase().as_str() {
        "claude" | "codex" | "gemini" => Ok(()),
        _ => Err(format!("Invalid AI type '{}'. Must be one of: claude, codex, gemini", ai_type))
    }
}
