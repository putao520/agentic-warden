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
    handler::server::tool::{Parameters, ToolRouter},
    tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;

// Note: ProviderManager is managed by supervisor module, not directly by MCP server

// ==================== 参数结构体定义 ====================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StartConcurrentTasksParams {
    pub tasks_json: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetTaskCommandParams {
    pub ai_type: String,
    pub task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchHistoryParams {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionTodosParams {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>, // Pending, InProgress, Completed, Cancelled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

/// Agentic-Warden MCP服务器
///
/// 使用rmcp库实现标准的MCP v1.0服务器，提供以下工具：
/// - `start_concurrent_tasks`: 并发启动多个任务
/// - `get_task_command`: 获取单个任务的启动命令
/// - `search_history`: 查询历史对话（带session_id）
/// - `get_session_todos`: 通过session_id查询未完成TODO
#[derive(Clone)]
pub struct AgenticWardenMcpServer {
    /// 工具路由器（由rmcp宏生成）
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AgenticWardenMcpServer {
    /// 创建新的MCP服务器实例
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// 启动MCP服务器 (使用rmcp的标准stdio传输)
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🚀 Starting Agentic-Warden MCP Server");
        eprintln!("   Protocol: MCP v1.0 (rmcp-based)");
        eprintln!("   Transport: stdio");
        eprintln!("   Tools: 4 available");
        eprintln!();
        eprintln!("✓ Server ready. Use Ctrl+C to stop.");
        eprintln!();

        // 使用rmcp的标准stdio服务
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        self.serve(transport).await?.waiting().await?;

        eprintln!("✓ MCP server stopped gracefully");
        Ok(())
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
    pub async fn start_concurrent_tasks(
        &self,
        params: Parameters<StartConcurrentTasksParams>,
    ) -> String {
        // 解析tasks数组
        let tasks: Vec<Value> = match serde_json::from_str(&params.0.tasks_json) {
            Ok(v) => v,
            Err(e) => {
                return serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": format!("Invalid JSON format: {}", e)
                }))
                .unwrap();
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
                format!(
                    "agent {} -p {} '{}'",
                    ai_type,
                    p,
                    task.replace("'", "'\\''")
                )
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

        let success_count = results
            .iter()
            .filter(|r| r.get("success") == Some(&serde_json::json!(true)))
            .count();

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
    pub async fn get_task_command(&self, params: Parameters<GetTaskCommandParams>) -> String {
        let ai_type = params.0.ai_type;
        let task = params.0.task;
        let provider = params.0.provider;

        // 验证AI类型
        if let Err(e) = parse_ai_type(&ai_type) {
            return serde_json::to_string_pretty(&serde_json::json!({
                "success": false,
                "error": format!("{}", e)
            }))
            .unwrap();
        }

        // 构建命令字符串
        let command = if let Some(ref p) = provider {
            format!(
                "agent {} -p {} '{}'",
                ai_type,
                p,
                task.replace("'", "'\\''")
            )
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
        }))
        .unwrap()
    }

    /// Search for relevant conversation history using semantic similarity, optionally filtered by session_id
    ///
    /// # 参数
    /// - `query`: 搜索查询
    /// - `session_id`: 可选的会话ID过滤器
    /// - `limit`: 返回结果数量限制
    /// - `score_threshold`: 相似度阈值
    ///
    /// # 返回
    /// - success: 是否成功
    /// - conversations: 相关对话列表，包含：
    ///   - session_id: 会话ID
    ///   - content: 对话内容
    ///   - timestamp: 时间戳
    ///   - score: 相似度分数
    ///   - metadata: 元数据
    /// - message: 状态信息
    #[tool(
        description = "Search for relevant conversation history using semantic similarity, optionally filtered by session_id"
    )]
    pub async fn search_history(&self, params: Parameters<SearchHistoryParams>) -> String {
        // 初始化内存管理器
        let memory_manager = match aiw::memory::MemoryManager::new().await {
            Ok(mm) => mm,
            Err(e) => {
                return serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": format!("Failed to initialize memory manager: {}", e),
                    "conversations": Vec::<Value>::new()
                }))
                .unwrap();
            }
        };

        // 搜索历史对话
        let limit = params.0.limit.unwrap_or(10);
        let score_threshold = params.0.score_threshold.unwrap_or(0.5) as f32;

        match memory_manager
            .search_relevant_memories(&params.0.query, Some(limit))
            .await
        {
            Ok(results) => {
                // 转换结果为对话格式，包含session_id
                let conversations: Vec<Value> = results
                    .into_iter()
                    .filter_map(|result| {
                        // 从metadata中提取session_id
                        let session_id = result
                            .point
                            .metadata
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        // 应用session_id过滤器
                        if let Some(ref filter_session_id) = params.0.session_id {
                            if session_id != *filter_session_id {
                                return None;
                            }
                        }

                        // 应用score_threshold过滤器
                        if result.score < score_threshold {
                            return None;
                        }

                        Some(serde_json::json!({
                            "session_id": session_id,
                            "content": result.point.content,
                            "timestamp": result.point.timestamp.to_rfc3339(),
                            "score": result.score,
                            "metadata": result.point.metadata
                        }))
                    })
                    .collect();

                serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "conversations": conversations,
                    "count": conversations.len(),
                    "query": params.0.query,
                    "message": "History search completed successfully"
                }))
                .unwrap()
            }
            Err(e) => serde_json::to_string_pretty(&serde_json::json!({
                "success": false,
                "error": format!("Failed to search history: {}", e),
                "conversations": Vec::<Value>::new()
            }))
            .unwrap(),
        }
    }

    /// Get TODOs for a specific session_id, optionally filtered by status
    ///
    /// # 参数
    /// - `session_id`: 会话ID
    /// - `status`: 可选的状态过滤器 (Pending, InProgress, Completed, Cancelled)
    /// - `limit`: 可选的结果数量限制
    ///
    /// # 返回
    /// - success: 是否成功
    /// - todos: TODO列表，包含：
    ///   - id: TODO ID
    ///   - title: 标题
    ///   - description: 描述
    ///   - status: 状态
    ///   - priority: 优先级
    ///   - created_at: 创建时间
    ///   - updated_at: 更新时间
    ///   - tags: 标签
    ///   - metadata: 元数据
    ///   - session_id: 会话ID
    /// - message: 状态信息
    #[tool(description = "Get TODOs for a specific session_id, optionally filtered by status")]
    pub async fn get_session_todos(&self, params: Parameters<GetSessionTodosParams>) -> String {
        // 初始化内存管理器
        let memory_manager = match aiw::memory::MemoryManager::new().await {
            Ok(mm) => mm,
            Err(e) => {
                return serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": format!("Failed to initialize memory manager: {}", e),
                    "todos": Vec::<Value>::new()
                }))
                .unwrap();
            }
        };

        // 解析状态过滤器
        let status_filter = if let Some(ref status_str) = params.0.status {
            match status_str.to_lowercase().as_str() {
                "pending" => Some(aiw::memory::todo_manager::TodoStatus::Pending),
                "inprogress" => Some(aiw::memory::todo_manager::TodoStatus::InProgress),
                "completed" => Some(aiw::memory::todo_manager::TodoStatus::Completed),
                "cancelled" => Some(aiw::memory::todo_manager::TodoStatus::Cancelled),
                _ => None,
            }
        } else {
            None
        };

        // 使用get_todos_by_session_id方法查找特定session_id的TODO
        match memory_manager
            .get_todos_by_session_id(&params.0.session_id, status_filter)
            .await
        {
            Ok(todos) => {
                let todos_json: Vec<Value> = todos
                    .into_iter()
                    .map(|todo| {
                        // 从metadata中提取session_id
                        let session_id = todo
                            .metadata
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        serde_json::json!({
                            "id": todo.id,
                            "title": todo.title,
                            "description": todo.description,
                            "status": format!("{:?}", todo.status),
                            "priority": format!("{:?}", todo.priority),
                            "created_at": todo.created_at.to_rfc3339(),
                            "updated_at": todo.updated_at.to_rfc3339(),
                            "tags": todo.tags,
                            "metadata": todo.metadata,
                            "session_id": session_id
                        })
                    })
                    .collect();

                serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "todos": todos_json,
                    "count": todos_json.len(),
                    "session_id": params.0.session_id,
                    "message": "Session TODOs retrieved successfully"
                }))
                .unwrap()
            }
            Err(e) => serde_json::to_string_pretty(&serde_json::json!({
                "success": false,
                "error": format!("Failed to get session todos: {}", e),
                "todos": Vec::<Value>::new()
            }))
            .unwrap(),
        }
    }
}

// 使用tool_handler宏实现ServerHandler trait
#[tool_handler(router = self.tool_router)]
impl ServerHandler for AgenticWardenMcpServer {}

// ==================== 辅助函数 ====================

/// 解析AI类型字符串
fn parse_ai_type(ai_type: &str) -> Result<(), String> {
    match ai_type.to_lowercase().as_str() {
        "claude" | "codex" | "gemini" => Ok(()),
        _ => Err(format!(
            "Invalid AI type '{}'. Must be one of: claude, codex, gemini",
            ai_type
        )),
    }
}
