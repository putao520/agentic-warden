//! MCP (Model Context Protocol) 服务器模块
//!
//! 提供Agentic-Warden的MCP服务，允许外部AI CLI通过MCP协议调用功能

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::provider::manager::ProviderManager;

/// Agentic-Warden MCP服务器
#[derive(Clone)]
pub struct AgenticWardenMcpServer {
    /// Provider管理器
    provider_manager: Arc<Mutex<ProviderManager>>,
}

impl AgenticWardenMcpServer {
    /// 创建新的MCP服务器实例
    pub fn new(provider_manager: ProviderManager) -> Self {
        Self {
            provider_manager: Arc::new(Mutex::new(provider_manager)),
        }
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

    /// 启动AI CLI并执行任务
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

    // 简化实现 - 只返回当前进程信息作为示例
    let current_pid = std::process::id();
    let current_name = std::env::args().next().unwrap_or_default();
    let ai_cli_type = if is_ai_cli_process(&current_name) {
        Some(detect_ai_cli_type(&current_name))
    } else {
        None
    };

    processes.push(ProcessInfo {
        pid: current_pid,
        name: current_name,
        status: "running".to_string(),
        command: "agentic-warden".to_string(),
        parent_pid: None,
        ai_cli_type,
    });

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