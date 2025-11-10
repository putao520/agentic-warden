//! MCP (Model Context Protocol) 服务器模块
//!
//! 提供Agentic-Warden的MCP服务，允许外部AI CLI通过MCP协议调用功能
//!
//! # 任务管理
//!
//! MCP服务器使用工厂模式获取进程内任务注册表：
//! - 通过 `RegistryFactory::instance().get_mcp_registry()` 获取全局唯一的MCP注册表
//! - 所有MCP启动的任务都存储在这个注册表中
//! - 与CLI任务注册表完全隔离

use std::sync::Arc;
use std::ffi::OsString;
use tokio::sync::Mutex;
use crate::provider::manager::ProviderManager;
use crate::supervisor;

// 使用工厂模式获取注册表
use agentic_warden::registry_factory::{RegistryFactory, TaskSource};
use agentic_warden::process_registry::InProcessRegistry;

/// Agentic-Warden MCP服务器
/// 使用工厂模式获取进程内任务注册表，确保全局唯一实例
#[derive(Clone)]
pub struct AgenticWardenMcpServer {
    /// Provider管理器
    provider_manager: Arc<Mutex<ProviderManager>>,
}

impl AgenticWardenMcpServer {
    /// 创建新的MCP服务器实例
    ///
    /// 注意：任务注册表通过RegistryFactory获取，确保全局唯一
    pub fn new(provider_manager: ProviderManager) -> Self {
        Self {
            provider_manager: Arc::new(Mutex::new(provider_manager)),
        }
    }

    /// 获取MCP任务注册表（从工厂获取全局单例）
    pub fn registry(&self) -> Arc<InProcessRegistry> {
        RegistryFactory::instance().get_mcp_registry()
    }

    /// 运行MCP服务器
    pub async fn run_stdio_server(self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("Starting Agentic-Warden MCP server with stdio transport...");
        eprintln!("Available tools:");
        eprintln!("  - monitor_processes: Monitor AI CLI processes");
        eprintln!("  - get_process_tree: Get process tree information");
        eprintln!("  - get_provider_status: Get provider configuration");
        eprintln!("  - start_ai_cli: Start AI CLI with prompt");
        eprintln!();
        eprintln!("MCP server is running. Use Ctrl+C to stop.");

        // 简单的MCP服务器实现 - 读取JSON-RPC请求并响应
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let mut reader = BufReader::new(tokio::io::stdin());
        let mut writer = tokio::io::stdout();

        let mut buffer = String::new();

        loop {
            buffer.clear();

            // 读取一行输入
            match reader.read_line(&mut buffer).await {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    break;
                }
            }

            let line = buffer.trim();
            if line.is_empty() {
                continue;
            }

            // 解析JSON-RPC请求
            match self.handle_mcp_request(line).await {
                Ok(response) => {
                    if let Err(e) = writer.write_all(response.as_bytes()).await {
                        eprintln!("Error writing response: {}", e);
                        break;
                    }
                    if let Err(e) = writer.write_all(b"\n").await {
                        eprintln!("Error writing newline: {}", e);
                        break;
                    }
                    if let Err(e) = writer.flush().await {
                        eprintln!("Error flushing: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error handling request: {}", e);
                }
            }
        }

        eprintln!("MCP server stopped");
        Ok(())
    }

    /// 处理MCP请求
    async fn handle_mcp_request(&self, request: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 简单的JSON-RPC响应
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "message": "Agentic-Warden MCP server is running",
                "tools": [
                    {
                        "name": "monitor_processes",
                        "description": "Monitor AI CLI processes"
                    },
                    {
                        "name": "get_process_tree",
                        "description": "Get process tree information"
                    },
                    {
                        "name": "get_provider_status",
                        "description": "Get provider configuration"
                    },
                    {
                        "name": "start_ai_cli",
                        "description": "Start AI CLI with prompt"
                    }
                ]
            }
        });

        Ok(serde_json::to_string(&response)?)
    }

    /// 监控所有运行的AI CLI进程及其状态
    pub async fn monitor_processes(&self, filter: Option<String>, ai_only: Option<bool>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        match get_system_processes().await {
            Ok(processes) => {
                let mut filtered_processes = Vec::new();

                for process in processes {
                    // 应用过滤器
                    if let Some(filter_str) = &filter {
                        if !process.name.to_lowercase().contains(&filter_str.to_lowercase()) {
                            continue;
                        }
                    }

                    // 如果只显示AI CLI进程
                    if ai_only.unwrap_or(false) {
                        if !is_ai_cli_process(&process.name) {
                            continue;
                        }
                    }

                    filtered_processes.push(process);
                }

                let process_json: Vec<serde_json::Value> = filtered_processes.into_iter().map(|process| {
                    serde_json::json!({
                        "pid": process.pid,
                        "name": process.name,
                        "status": process.status,
                        "command": process.command,
                        "parent_pid": process.parent_pid,
                        "ai_cli_type": process.ai_cli_type,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })
                }).collect();

                Ok(serde_json::json!({
                    "processes": process_json,
                    "count": process_json.len(),
                    "message": format!("Found {} processes", process_json.len())
                }))
            }
            Err(e) => {
                Ok(serde_json::json!({
                    "error": format!("Failed to get processes: {}", e),
                    "processes": [],
                    "count": 0
                }))
            }
        }
    }

    /// 获取Provider状态和配置信息
    pub async fn get_provider_status(&self, provider_id: Option<String>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let provider_manager = self.provider_manager.lock().await;

        let status = if let Some(id) = provider_id {
            // 获取特定Provider状态
            let providers_config = provider_manager.get_providers_config();
            if providers_config.providers.contains_key(&id) {
                serde_json::json!({
                    "provider_id": id,
                    "provider": providers_config.providers.get(&id),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            } else {
                serde_json::json!({
                    "error": format!("Provider '{}' not found", id)
                })
            }
        } else {
            // 获取所有Provider状态
            let providers_config = provider_manager.get_providers_config();
            serde_json::json!({
                "default_provider": providers_config.default_provider,
                "providers": providers_config.providers,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
        };

        Ok(status)
    }

    /// 启动AI CLI并执行任务（旧方法，保留兼容性）
    pub async fn start_ai_cli(
        &self,
        ai_type: String,
        provider: Option<String>,
        prompt: String,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // 验证AI类型
        let ai_cli_type = parse_ai_type(&ai_type)?;

        // 构建外部AI CLI命令
        match start_external_ai_cli(ai_cli_type, provider, &prompt).await {
            Ok(output) => {
                Ok(serde_json::json!({
                    "success": true,
                    "output": output,
                    "message": "AI CLI execution completed"
                }))
            }
            Err(e) => {
                Ok(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to start AI CLI: {}", e),
                    "message": "AI CLI execution failed"
                }))
            }
        }
    }

    /// 并发启动多个AI CLI任务
    ///
    /// 直接在MCP服务器中启动所有AI CLI进程，每个任务在后台运行。
    /// 使用supervisor::execute_cli复用现有实现，包括：
    /// - Provider配置和验证
    /// - 环境变量注入
    /// - TaskRegistry自动注册
    /// - 日志文件管理
    /// - 进程组管理
    ///
    /// # Arguments
    /// * `tasks` - 任务列表，每个任务包含ai_type、provider、task内容
    ///
    /// # Returns
    /// 返回包含wait命令的JSON对象，用于等待所有任务完成
    pub async fn start_concurrent_tasks(
        &self,
        tasks: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        if tasks.is_empty() {
            return Ok(serde_json::json!({
                "success": false,
                "error": "No tasks provided"
            }));
        }

        let mut started_count = 0;
        let mut errors = Vec::new();

        // 并发启动所有任务
        for (idx, task_json) in tasks.iter().enumerate() {
            let ai_type_str = task_json.get("ai_type")
                .and_then(|v| v.as_str())
                .ok_or(format!("Task {}: Missing ai_type", idx))?;

            let provider = task_json.get("provider")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let task_content = task_json.get("task")
                .and_then(|v| v.as_str())
                .ok_or(format!("Task {}: Missing task", idx))?;

            // 解析AI类型
            let cli_type = match parse_ai_type(ai_type_str) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(format!("Task {}: {}", idx, e));
                    continue;
                }
            };

            // 构建参数（任务内容）
            let args = vec![OsString::from(task_content)];

            // 在后台启动任务
            let registry = self.registry.clone();
            tokio::spawn(async move {
                if let Err(e) = supervisor::execute_cli(&registry, &cli_type, &args, provider).await {
                    eprintln!("Task {} failed: {}", idx, e);
                }
            });

            started_count += 1;
        }

        if started_count == 0 {
            return Ok(serde_json::json!({
                "success": false,
                "error": "No tasks started",
                "errors": errors
            }));
        }

        Ok(serde_json::json!({
            "success": true,
            "started_count": started_count,
            "task": {
                "description": format!("Wait for {} concurrent AI CLI tasks to complete", started_count),
                "tool": "bash",
                "command": "agentic-warden wait --timeout 12h",
                "timeout_ms": 43200000  // 12 hours in milliseconds
            },
            "message": format!(
                "Started {} concurrent AI CLI tasks in background. Execute the 'task' using Bash tool.",
                started_count
            ),
            "errors": if errors.is_empty() { serde_json::Value::Null } else { serde_json::json!(errors) }
        }))
    }

    /// 获取普通启动命令
    ///
    /// 返回符合Claude Code标准的task JSON结构，让Claude Code通过Bash工具执行。
    ///
    /// # Arguments
    /// * `ai_type` - AI CLI类型（codex, gemini等）
    /// * `task` - 任务内容
    /// * `provider` - 可选的Provider
    ///
    /// # Returns
    /// 返回符合Claude Code标准的task JSON对象
    pub async fn get_task_command(
        &self,
        ai_type: String,
        task: String,
        provider: Option<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // 验证AI类型
        let _ = parse_ai_type(&ai_type)?;

        // 构建命令字符串
        let command = if let Some(p) = &provider {
            format!("agent {} -p {} '{}'", ai_type, p, task.replace("'", "'\\''"))
        } else {
            format!("agent {} '{}'", ai_type, task.replace("'", "'\\''"))
        };

        // 构建任务描述
        let description = if let Some(p) = &provider {
            format!("Execute {} task with provider {}: {}", ai_type, p, task)
        } else {
            format!("Execute {} task: {}", ai_type, task)
        };

        Ok(serde_json::json!({
            "success": true,
            "task": {
                "description": description,
                "tool": "bash",
                "command": command,
                "timeout_ms": 43200000  // 12 hours in milliseconds
            },
            "ai_type": ai_type,
            "provider": provider,
            "message": "Execute the 'task' using Bash tool with 12h timeout"
        }))
    }

    /// 获取指定进程的进程树信息
    ///
    /// 使用agentic-warden的进程树追踪功能获取完整的进程层级结构。
    /// 特别标注AI CLI进程，帮助理解进程之间的父子关系。
    ///
    /// # Arguments
    /// * `pid` - 目标进程ID
    ///
    /// # Returns
    /// 包含进程链、AI CLI识别信息的JSON对象
    pub async fn get_process_tree(&self, pid: u32) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        match crate::core::process_tree::get_process_tree(pid) {
            Ok(tree_info) => {
                // 获取进程链中每个进程的详细信息
                let mut process_chain_details = Vec::new();

                for &chain_pid in &tree_info.process_chain {
                    if let Ok(processes) = get_system_processes().await {
                        if let Some(process) = processes.iter().find(|p| p.pid == chain_pid) {
                            process_chain_details.push(serde_json::json!({
                                "pid": process.pid,
                                "name": process.name,
                                "command": process.command,
                                "ai_cli_type": process.ai_cli_type,
                                "is_ai_cli": process.ai_cli_type.is_some()
                            }));
                        } else {
                            // 进程可能已经退出，仅记录PID
                            process_chain_details.push(serde_json::json!({
                                "pid": chain_pid,
                                "name": "unknown",
                                "status": "not_found"
                            }));
                        }
                    }
                }

                Ok(serde_json::json!({
                    "success": true,
                    "process_tree": {
                        "process_chain": tree_info.process_chain,
                        "root_parent_pid": tree_info.root_parent_pid,
                        "depth": tree_info.depth,
                        "has_ai_cli_root": tree_info.has_ai_cli_root,
                        "ai_cli_type": tree_info.ai_cli_type,
                        "process_details": process_chain_details
                    },
                    "message": format!("Process tree depth: {} levels", tree_info.depth)
                }))
            }
            Err(e) => {
                Ok(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to get process tree: {}", e),
                    "message": "Process tree unavailable"
                }))
            }
        }
    }

    /// 安全终止指定的进程
    ///
    /// 使用两阶段终止策略：
    /// 1. 首先发送SIGTERM，给进程机会优雅退出
    /// 2. 如果进程仍然存在，发送SIGKILL强制终止
    ///
    /// 安全检查：
    /// - 只允许终止AI CLI相关进程
    /// - 阻止终止agentic-warden自身
    /// - 阻止终止系统关键进程
    ///
    /// # Arguments
    /// * `pid` - 要终止的进程ID
    /// * `force` - 是否跳过SIGTERM直接使用SIGKILL
    ///
    /// # Returns
    /// 包含终止结果的JSON对象
    pub async fn terminate_process(&self, pid: u32, force: Option<bool>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // 安全检查：不允许终止自身
        if pid == std::process::id() {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Cannot terminate agentic-warden itself",
                "message": "Operation denied for safety"
            }));
        }

        // 获取进程信息进行验证
        match get_system_processes().await {
            Ok(processes) => {
                let process = processes.iter().find(|p| p.pid == pid);

                if let Some(process) = process {
                    // 安全检查：只允许终止AI CLI进程
                    if !is_ai_cli_process(&process.name) && !is_ai_cli_process(&process.command) {
                        return Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Process '{}' (PID: {}) is not an AI CLI process", process.name, pid),
                            "message": "Only AI CLI processes can be terminated via MCP"
                        }));
                    }

                    // 执行终止操作
                    let force_kill = force.unwrap_or(false);

                    #[cfg(target_family = "unix")]
                    {
                        use nix::sys::signal::{self, Signal};
                        use nix::unistd::Pid;

                        let nix_pid = Pid::from_raw(pid as i32);

                        if force_kill {
                            // 直接发送SIGKILL
                            match signal::kill(nix_pid, Signal::SIGKILL) {
                                Ok(_) => {
                                    Ok(serde_json::json!({
                                        "success": true,
                                        "pid": pid,
                                        "process_name": process.name,
                                        "method": "SIGKILL",
                                        "message": "Process forcefully terminated"
                                    }))
                                }
                                Err(e) => {
                                    Ok(serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to terminate process: {}", e),
                                        "message": "Termination failed"
                                    }))
                                }
                            }
                        } else {
                            // 优雅终止：先SIGTERM，等待，再SIGKILL
                            match signal::kill(nix_pid, Signal::SIGTERM) {
                                Ok(_) => {
                                    // 等待2秒让进程优雅退出
                                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                                    // 检查进程是否还存在
                                    let still_alive = get_system_processes()
                                        .await
                                        .ok()
                                        .and_then(|procs| procs.iter().find(|p| p.pid == pid).cloned())
                                        .is_some();

                                    if still_alive {
                                        // 进程仍然存在，发送SIGKILL
                                        let _ = signal::kill(nix_pid, Signal::SIGKILL);
                                        Ok(serde_json::json!({
                                            "success": true,
                                            "pid": pid,
                                            "process_name": process.name,
                                            "method": "SIGTERM -> SIGKILL",
                                            "message": "Process terminated (required SIGKILL)"
                                        }))
                                    } else {
                                        Ok(serde_json::json!({
                                            "success": true,
                                            "pid": pid,
                                            "process_name": process.name,
                                            "method": "SIGTERM",
                                            "message": "Process gracefully terminated"
                                        }))
                                    }
                                }
                                Err(e) => {
                                    Ok(serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to send SIGTERM: {}", e),
                                        "message": "Termination failed"
                                    }))
                                }
                            }
                        }
                    }

                    #[cfg(target_family = "windows")]
                    {
                        use std::process::Command;

                        // Windows: 使用taskkill
                        let force_flag = if force_kill { "/F" } else { "/T" };
                        let output = Command::new("taskkill")
                            .args(&[force_flag, "/PID", &pid.to_string()])
                            .output();

                        match output {
                            Ok(output) if output.status.success() => {
                                Ok(serde_json::json!({
                                    "success": true,
                                    "pid": pid,
                                    "process_name": process.name,
                                    "method": if force_kill { "taskkill /F" } else { "taskkill /T" },
                                    "message": "Process terminated"
                                }))
                            }
                            Ok(output) => {
                                Ok(serde_json::json!({
                                    "success": false,
                                    "error": String::from_utf8_lossy(&output.stderr).to_string(),
                                    "message": "Termination failed"
                                }))
                            }
                            Err(e) => {
                                Ok(serde_json::json!({
                                    "success": false,
                                    "error": format!("Failed to execute taskkill: {}", e),
                                    "message": "Termination failed"
                                }))
                            }
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": format!("Process with PID {} not found", pid),
                        "message": "Process does not exist"
                    }))
                }
            }
            Err(e) => {
                Ok(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to enumerate processes: {}", e),
                    "message": "Could not verify process"
                }))
            }
        }
    }
}

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
        use psutil::process::Process;

        // 获取所有进程
        for proc in psutil::process::processes()? {
            if let Ok(proc) = proc {
                let pid = proc.pid();

                // 尝试获取进程名称
                let name = proc.name().unwrap_or_else(|_| String::from("unknown"));

                // 尝试获取命令行
                let command = match proc.cmdline() {
                    Ok(Some(args)) => args, // cmdline()返回String
                    _ => name.clone(),
                };

                // 尝试获取状态
                let status = proc.status()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|_| "unknown".to_string());

                // 尝试获取父进程ID
                let parent_pid = proc.ppid().ok().flatten();

                // 检测AI CLI类型
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

        for (pid, process) in sys.processes() {
            let name = process.name().to_string();
            let command = process.cmd().join(" ");
            let status = format!("{:?}", process.status());
            let parent_pid = process.parent().map(|p| p.as_u32());

            // 检测AI CLI类型
            let ai_cli_type = if is_ai_cli_process(&name) || is_ai_cli_process(&command) {
                Some(detect_ai_cli_type(&name))
            } else {
                None
            };

            processes.push(ProcessInfo {
                pid: pid.as_u32(),
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

/// 启动外部AI CLI
async fn start_external_ai_cli(
    ai_type: crate::cli_type::CliType,
    provider: Option<String>,
    prompt: &str,
) -> Result<String, anyhow::Error> {
    use tokio::process::Command;

    let ai_cmd = match ai_type {
        crate::cli_type::CliType::Claude => "claude",
        crate::cli_type::CliType::Codex => "codex",
        crate::cli_type::CliType::Gemini => "gemini",
    };

    let mut cmd = Command::new(ai_cmd);
    cmd.arg(prompt);

    if let Some(provider_name) = provider {
        cmd.arg("-p").arg(provider_name);
    }

    let output = cmd.output().await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(anyhow::anyhow!(
            "AI CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// 判断是否为AI CLI进程
fn is_ai_cli_process(process_name: &str) -> bool {
    let process_name_lower = process_name.to_lowercase();
    process_name_lower.contains("claude")
        || process_name_lower.contains("codex")
        || process_name_lower.contains("gemini")
        || process_name_lower.contains("agentic-warden")
}

/// 检测AI CLI类型
fn detect_ai_cli_type(process_name: &str) -> String {
    let name_lower = process_name.to_lowercase();
    if name_lower.contains("claude") {
        "claude".to_string()
    } else if name_lower.contains("codex") {
        "codex".to_string()
    } else if name_lower.contains("gemini") {
        "gemini".to_string()
    } else {
        "unknown".to_string()
    }
}

/// 解析AI类型
fn parse_ai_type(ai_type: &str) -> Result<crate::cli_type::CliType, String> {
    match ai_type.to_lowercase().as_str() {
        "claude" => Ok(crate::cli_type::CliType::Claude),
        "codex" => Ok(crate::cli_type::CliType::Codex),
        "gemini" => Ok(crate::cli_type::CliType::Gemini),
        _ => Err(format!("Unsupported AI type: {}. Supported: claude, codex, gemini", ai_type)),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ai_cli_process() {
        assert!(is_ai_cli_process("claude"));
        assert!(is_ai_cli_process("claude.exe"));
        assert!(is_ai_cli_process("codex"));
        assert!(is_ai_cli_process("gemini"));
        assert!(!is_ai_cli_process("explorer"));
        assert!(!is_ai_cli_process("chrome"));
    }

    #[test]
    fn test_detect_ai_cli_type() {
        assert_eq!(detect_ai_cli_type("claude"), "claude");
        assert_eq!(detect_ai_cli_type("claude.exe"), "claude");
        assert_eq!(detect_ai_cli_type("codex"), "codex");
        assert_eq!(detect_ai_cli_type("gemini"), "gemini");
        assert_eq!(detect_ai_cli_type("unknown"), "unknown");
    }

    #[test]
    fn test_parse_ai_type() {
        assert!(parse_ai_type("claude").is_ok());
        assert!(parse_ai_type("CODEX").is_ok());
        assert!(parse_ai_type("gemini").is_ok());
        assert!(parse_ai_type("invalid").is_err());
    }
}