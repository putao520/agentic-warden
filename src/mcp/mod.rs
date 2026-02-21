pub mod capability_detector;
mod js_executor;
pub use js_executor::{JsExecutionReport, JsToolExecutor};

use crate::platform;
use crate::registry_factory::RegistryFactory;
use crate::task_record::{TaskStatus, WorktreeInfo};
use anyhow::Error;
use chrono::{DateTime, Utc};

use crate::mcp_routing::js_orchestrator::{BoaRuntimePool, McpFunctionInjector};
use crate::mcp_routing::registry::{DynamicToolRegistry, RegisteredTool, RegistryConfig};
use crate::mcp_routing::{
    models::{IntelligentRouteRequest, IntelligentRouteResponse},
    IntelligentRouter,
};
use crate::roles::{builtin::get_builtin_role, builtin::list_builtin_roles, RoleManager, Role, RoleInfo};
use capability_detector::ClientCapabilities;
use rmcp::{
    handler::server::tool::{ToolCallContext, ToolRouter},
    handler::server::wrapper::Parameters,
    handler::server::ServerHandler,
    model::{Implementation, InitializeRequestParam, InitializeResult, ServerCapabilities, Tool},
    service::{RequestContext, RoleServer},
    tool, Json, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct StartTaskParams {
    /// AI CLI type (claude, codex, or gemini).
    pub ai_type: String,
    /// Task description/prompt for the AI.
    pub task: String,
    /// Optional provider name to use for this task.
    ///
    /// All available providers and their scenarios are defined in ~/.aiw/providers.json.
    /// Each provider can have a 'scenario' field describing when to use it.
    ///
    /// If not specified, the default_provider from configuration will be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Optional role name to inject from ~/.aiw/role directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Optional working directory for the AI CLI process.
    /// If specified, the AI CLI will be started in this directory.
    /// The directory must exist and be a valid directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Extra CLI arguments to pass through to the underlying AI CLI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_args: Option<Vec<String>>,
    /// Whether to create a git worktree for isolated execution (default: false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TaskLaunchInfo {
    /// UUID task identifier for external consumers.
    pub task_id: String,
    /// Process ID of the launched task.
    pub pid: u32,
    /// Path to the task log file.
    pub log_file: String,
    /// Task status in registry.
    pub status: TaskStatus,
    /// Task start time.
    pub started_at: DateTime<Utc>,
    /// AI CLI type used.
    pub ai_type: String,
    /// Original task prompt (without role prefix).
    pub task: String,
    /// Provider used (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Worktree isolation info (if worktree=true was requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_info: Option<WorktreeInfo>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TaskInfo {
    /// UUID task identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Process ID.
    pub pid: u32,
    /// Log file path.
    pub log_file: String,
    /// Registry task status.
    pub status: TaskStatus,
    /// Task start time.
    pub started_at: DateTime<Utc>,
    /// Optional completion time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Cleanup reason if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_reason: Option<String>,
    /// Manager PID that launched the task (if tracked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manager_pid: Option<u32>,
    /// Exit code if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Log identifier stored in registry.
    pub log_id: String,
    /// Task result string (if completed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Worktree isolation info.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_info: Option<WorktreeInfo>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct StopTaskParams {
    /// UUID task identifier of the task to stop.
    pub task_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct StopTaskResult {
    /// Whether stop succeeded.
    pub success: bool,
    /// Human-readable status message.
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct GetTaskLogsParams {
    /// UUID task identifier whose logs should be retrieved.
    pub task_id: String,
    /// Optional number of lines to tail from the end of the log.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail_lines: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TaskLogsResult {
    /// UUID task identifier.
    pub task_id: String,
    /// PID associated with the log file.
    pub pid: u32,
    /// Log file path.
    pub log_file: String,
    /// Content of the log (entire file or tail).
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct GetTaskStatusParams {
    /// UUID task identifier to query.
    pub task_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TaskStatusResult {
    /// UUID task identifier.
    pub task_id: String,
    /// Process ID.
    pub pid: u32,
    /// Registry task status.
    pub status: TaskStatus,
    /// Whether the process is currently alive.
    pub process_alive: bool,
    /// Exit code if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Task result string (if completed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Worktree isolation info.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_info: Option<WorktreeInfo>,
    /// Task start time.
    pub started_at: DateTime<Utc>,
    /// Optional completion time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Log file path.
    pub log_file: String,
}

/// Detect user's preferred language based on system locale
fn detect_language() -> String {
    if let Some(locale) = sys_locale::get_locale() {
        if locale.starts_with("zh") {
            return "zh-CN".to_string();
        }
    }
    "en".to_string()
}

/// Load a single role (user-defined first, then builtin)
fn load_single_role_for_mcp(name: &str, lang: &str) -> Option<Role> {
    // Try user-defined roles first (allows overriding built-in roles)
    if let Ok(manager) = RoleManager::new() {
        if let Ok(role) = manager.get_role(name) {
            return Some(role);
        }
    }
    // Fall back to built-in roles
    if let Ok(role) = get_builtin_role(name, lang) {
        return Some(role);
    }
    None
}

/// Load multiple roles
/// Returns: (valid_roles, invalid_names)
fn load_roles_for_mcp(names: &[&str], lang: &str) -> (Vec<Role>, Vec<String>) {
    let mut valid_roles = Vec::new();
    let mut invalid_names = Vec::new();

    for name in names {
        if let Some(role) = load_single_role_for_mcp(name, lang) {
            valid_roles.push(role);
        } else {
            invalid_names.push(name.to_string());
        }
    }

    (valid_roles, invalid_names)
}

async fn wait_for_registry_entry(
    registry: &crate::registry_factory::McpRegistry,
    existing: &HashSet<u32>,
) -> Result<Option<crate::storage::RegistryEntry>, String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        let entries = registry.entries().map_err(|e| e.to_string())?;
        if let Some(new_entry) = entries
            .into_iter()
            .find(|entry| !existing.contains(&entry.pid))
        {
            return Ok(Some(new_entry));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(None)
}

pub async fn start_task(params: StartTaskParams) -> Result<TaskLaunchInfo, String> {
    use crate::cli_type::parse_cli_type;
    use crate::supervisor;

    let task_id = uuid::Uuid::new_v4().to_string();
    let registry = RegistryFactory::instance().get_mcp_registry();

    let mut prompt = params.task.clone();

    if let Some(role_str) = &params.role {
        // Parse and deduplicate role names (preserve order)
        let mut seen = std::collections::HashSet::new();
        let role_names: Vec<&str> = role_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter(|s| seen.insert(*s))
            .collect();

        if !role_names.is_empty() {
            let lang = detect_language();
            let (valid_roles, invalid_names) = load_roles_for_mcp(&role_names, &lang);

            // Log warnings for invalid roles
            for name in &invalid_names {
                eprintln!("Warning: Role '{}' not found, skipping.", name);
            }

            // Combine valid roles or fallback to common
            if valid_roles.is_empty() {
                eprintln!("Warning: All specified roles not found, falling back to 'common' role.");
                if let Some(fallback) = load_single_role_for_mcp("common", &lang) {
                    prompt = format!("{}\n\n---\n\n{}", fallback.content, params.task);
                }
            } else {
                let role_contents: Vec<&str> = valid_roles.iter().map(|r| r.content.as_str()).collect();
                let combined = role_contents.join("\n\n---\n\n");
                prompt = format!("{}\n\n---\n\n{}", combined, params.task);
            }
        }
    }

    let cli_type = parse_cli_type(&params.ai_type).ok_or_else(|| {
        format!(
            "Invalid AI type: {}. Must be claude, codex, or gemini",
            params.ai_type
        )
    })?;

    // Handle worktree creation if requested
    let mut cwd = params.cwd.clone();
    let mut worktree_info: Option<WorktreeInfo> = None;
    if params.worktree == Some(true) {
        let work_dir = cwd
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));
        crate::worktree::check_git_repository(&work_dir)
            .map_err(|e| e.to_string())?;
        let (wt_path, branch, commit) = crate::worktree::create_worktree(&work_dir)
            .map_err(|e| e.to_string())?;
        let info = WorktreeInfo {
            path: wt_path.display().to_string(),
            branch,
            commit,
        };
        cwd = Some(wt_path.display().to_string());
        worktree_info = Some(info);
    }

    let existing: HashSet<u32> = registry
        .entries()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|entry| entry.pid)
        .collect();

    // Build args with cli_args passthrough
    let cli_args_ref: Vec<String> = params.cli_args.clone().unwrap_or_default();
    let args = cli_type.build_full_access_args_with_cli(&prompt, &cli_args_ref);
    let os_args: Vec<OsString> = args.into_iter().map(OsString::from).collect();

    let spawn_registry = registry.clone();
    let spawn_cli_type = cli_type.clone();
    let spawn_args = os_args.clone();
    let spawn_provider = params.provider.clone();
    let spawn_cwd = cwd.clone().map(PathBuf::from);

    tokio::spawn(async move {
        if let Err(err) = supervisor::execute_cli(
            &spawn_registry,
            &spawn_cli_type,
            &spawn_args,
            spawn_provider,
            spawn_cwd,
        )
        .await
        {
            eprintln!(
                "start_task: failed to launch {} task: {}",
                spawn_cli_type.display_name(),
                err
            );
        }
    });

    let new_entry = wait_for_registry_entry(&registry, &existing).await?;
    let entry = new_entry.ok_or_else(|| "Failed to register task in MCP registry".to_string())?;

    // Bind UUID and worktree info to the registry entry
    registry.update_task_metadata(entry.pid, task_id.clone(), worktree_info.clone());

    Ok(TaskLaunchInfo {
        task_id,
        pid: entry.pid,
        log_file: entry.record.log_path.clone(),
        status: entry.record.status.clone(),
        started_at: entry.record.started_at,
        ai_type: params.ai_type,
        task: params.task,
        provider: params.provider,
        worktree_info,
    })
}

fn registry_entry_to_task_info(entry: crate::storage::RegistryEntry) -> TaskInfo {
    TaskInfo {
        task_id: entry.record.task_id.clone(),
        pid: entry.pid,
        log_file: entry.record.log_path.clone(),
        status: entry.record.status.clone(),
        started_at: entry.record.started_at,
        completed_at: entry.record.completed_at,
        cleanup_reason: entry.record.cleanup_reason.clone(),
        manager_pid: entry.record.manager_pid,
        exit_code: entry.record.exit_code,
        log_id: entry.record.log_id.clone(),
        result: entry.record.result.clone(),
        worktree_info: entry.record.worktree_info.clone(),
    }
}

/// Resolve a task_id to (pid, TaskRecord). Shared by stop/logs/status handlers.
fn resolve_task_id(task_id: &str) -> Result<(u32, crate::task_record::TaskRecord), String> {
    let registry = RegistryFactory::instance().get_mcp_registry();
    registry
        .get_by_task_id(task_id)
        .ok_or_else(|| format!("task_id '{}' not found in MCP registry", task_id))
}

pub async fn list_tasks() -> Result<Vec<TaskInfo>, String> {
    let registry = RegistryFactory::instance().get_mcp_registry();
    let entries = registry.entries().map_err(|e| e.to_string())?;

    // Include all tasks (running + completed), not just alive processes
    Ok(entries
        .into_iter()
        .map(registry_entry_to_task_info)
        .collect())
}

pub async fn stop_task(params: StopTaskParams) -> Result<StopTaskResult, String> {
    let (pid, _record) = resolve_task_id(&params.task_id)?;
    let registry = RegistryFactory::instance().get_mcp_registry();

    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        kill(Pid::from_raw(pid as i32), Signal::SIGTERM).map_err(|e| e.to_string())?;
    }

    #[cfg(not(unix))]
    {
        platform::terminate_process(pid);
    }

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !platform::process_alive(pid) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    if platform::process_alive(pid) {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            kill(Pid::from_raw(pid as i32), Signal::SIGKILL).map_err(|e| e.to_string())?;
        }

        #[cfg(not(unix))]
        {
            platform::terminate_process(pid);
        }
    }

    registry
        .mark_completed(
            pid,
            Some("stopped_by_user".to_string()),
            None,
            Utc::now(),
        )
        .map_err(|e| e.to_string())?;

    Ok(StopTaskResult {
        success: true,
        message: format!("Task {} (pid {}) stopped", params.task_id, pid),
    })
}

pub async fn get_task_logs(params: GetTaskLogsParams) -> Result<TaskLogsResult, String> {
    let (pid, record) = resolve_task_id(&params.task_id)?;

    let log_path = PathBuf::from(record.log_path.clone());
    let content = if let Some(lines) = params.tail_lines {
        let data = fs::read_to_string(&log_path)
            .map_err(|e| format!("Failed to read log file {}: {}", log_path.display(), e))?;
        let all_lines: Vec<&str> = data.lines().collect();
        let start = all_lines.len().saturating_sub(lines);
        all_lines[start..].join("\n")
    } else {
        fs::read_to_string(&log_path)
            .map_err(|e| format!("Failed to read log file {}: {}", log_path.display(), e))?
    };

    Ok(TaskLogsResult {
        task_id: params.task_id,
        pid,
        log_file: record.log_path,
        content,
    })
}

pub async fn get_task_status(params: GetTaskStatusParams) -> Result<TaskStatusResult, String> {
    let (pid, record) = resolve_task_id(&params.task_id)?;

    Ok(TaskStatusResult {
        task_id: params.task_id,
        pid,
        status: record.status.clone(),
        process_alive: platform::process_alive(pid),
        exit_code: record.exit_code,
        result: record.result.clone(),
        worktree_info: record.worktree_info.clone(),
        started_at: record.started_at,
        completed_at: record.completed_at,
        log_file: record.log_path.clone(),
    })
}

// ===== list_roles / list_providers =====

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ListRolesResult {
    pub builtin_roles: Vec<String>,
    pub user_roles: Vec<RoleInfo>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ListProvidersResult {
    pub default_provider: String,
    pub providers: Vec<ProviderSummary>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ProviderSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatible_with: Option<Vec<String>>,
}

pub async fn list_roles() -> Result<ListRolesResult, String> {
    let builtin_roles = list_builtin_roles();
    let manager = RoleManager::new().map_err(|e| e.to_string())?;
    let user_roles = manager
        .list_all_roles()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| r.as_info())
        .collect();
    Ok(ListRolesResult {
        builtin_roles,
        user_roles,
    })
}

pub async fn list_providers() -> Result<ListProvidersResult, String> {
    let manager =
        crate::provider::manager::ProviderManager::new().map_err(|e| e.to_string())?;
    let default_name = manager
        .get_default_provider()
        .map(|(name, _)| name)
        .unwrap_or_default();
    let providers = manager
        .list_providers()
        .into_iter()
        .map(|(name, p)| ProviderSummary {
            name: name.clone(),
            scenario: p.scenario.clone(),
            compatible_with: p
                .compatible_with
                .as_ref()
                .map(|v| v.iter().map(|t| t.to_string()).collect()),
        })
        .collect();
    Ok(ListProvidersResult {
        default_provider: default_name,
        providers,
    })
}

#[derive(Clone)]
pub struct AgenticWardenMcpServer {
    router: Arc<IntelligentRouter>,
    tool_router: ToolRouter<Self>,
    // Client capability detection
    client_capabilities: Arc<RwLock<Option<ClientCapabilities>>>,
    // Dynamic tool registry (SSOT for MCP tools)
    tool_registry: Arc<DynamicToolRegistry>,
    // Store peer for sending notifications
    peer: Arc<RwLock<Option<rmcp::service::Peer<RoleServer>>>>,
    js_executor: Arc<JsToolExecutor>,
}

#[rmcp::tool_router(router = tool_router)]
impl AgenticWardenMcpServer {
    pub async fn bootstrap() -> Result<Self, String> {
        let router = IntelligentRouter::initialize()
            .await
            .map_err(|e| format!("Failed to initialise intelligent router: {e}"))?;
        let connection_pool = router.connection_pool();

        // Use router's shared registry and extend with server's base tools
        let registry = router
            .dynamic_registry()
            .ok_or_else(|| "Router dynamic registry not initialized".to_string())?;
        let tool_router = Self::tool_router();
        let base_tools = tool_router.list_all();
        registry.extend_base_tools(base_tools).await;

  
        // Initialize conversation history store
        let db_path = Self::get_history_db_path()
            .map_err(|e| format!("Failed to get history DB path: {e}"))?;
        let boa_pool = Arc::new(
            BoaRuntimePool::new()
                .await
                .map_err(|e| format!("Failed to initialize Boa runtime pool: {e}"))?,
        );
        let injector = Arc::new(McpFunctionInjector::new(connection_pool.clone()));
        let js_executor = Arc::new(JsToolExecutor::new(Arc::clone(&boa_pool), injector));

        // Start config file watcher for hot reload
        let config_path = dirs::home_dir()
            .ok_or_else(|| "Cannot find home directory".to_string())?
            .join(".aiw")
            .join("mcp.json");

        if config_path.exists() {
            use crate::mcp_routing::config_watcher;
            if let Err(e) = config_watcher::start_config_watcher(connection_pool, config_path).await
            {
                eprintln!("⚠️  Failed to start config watcher: {}", e);
            }
        }

        Ok(Self {
            router: Arc::new(router),
            tool_router,
            client_capabilities: Arc::new(RwLock::new(None)),
            tool_registry: registry,
            peer: Arc::new(RwLock::new(None)),
            js_executor,
        })
    }

    fn get_history_db_path() -> Result<PathBuf, String> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Failed to get config directory".to_string())?
            .join("aiw");

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;

        Ok(config_dir.join("conversation_history.db"))
    }

    /// Get all tool definitions (for testing and debugging)
    ///
    /// Returns all available MCP tools, including:
    /// - Base tools (intelligent_route, etc.)
    /// - Dynamic tools (JS orchestrated, proxied MCP)
    pub async fn get_all_tool_definitions(&self) -> Arc<Vec<Tool>> {
        self.tool_registry.get_all_tool_definitions().await
    }

    /// Get the count of dynamically registered tools (for testing FIFO eviction)
    pub async fn get_dynamic_tool_count(&self) -> usize {
        self.tool_registry.dynamic_tool_count().await
    }

    fn build_dynamic_tool_definition(
        name: &str,
        description: &str,
        input_schema: serde_json::Value,
    ) -> Tool {
        let schema_map = match input_schema {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };

        Tool {
            name: name.to_string().into(),
            title: None,
            description: Some(description.to_string().into()),
            input_schema: Arc::new(schema_map),
            output_schema: None,
            icons: None,
            annotations: None,
        }
    }

    #[tool(
        name = "intelligent_route",
        description = "Route user request to best MCP tool. Returns tool selection (LLM/vector). Auto-chooses execution mode (dynamic/query) based on client capabilities."
    )]
    pub async fn intelligent_route_tool(
        &self,
        params: Parameters<IntelligentRouteRequest>,
    ) -> Result<Json<IntelligentRouteResponse>, String> {
        use crate::mcp_routing::models::ExecutionMode;

        let mut request = params.0;

        // Auto-select execution mode based on client capabilities
        // (only if not explicitly overridden by caller)
        if request.execution_mode == ExecutionMode::Dynamic {
            if let Some(caps) = self.client_capabilities.read().await.as_ref() {
                if !caps.supports_dynamic_tools {
                    // Client doesn't support dynamic registration, use query mode
                    request.execution_mode = ExecutionMode::Query;
                    eprintln!(
                        "   ⚠️  Switching to Query mode (client doesn't support dynamic tools)"
                    );
                }
            }
        }

        let mut response = self
            .router
            .intelligent_route(request.clone())
            .await
            .map_err(|err| err.to_string())?;

        // Handle dynamic registration mode
        if request.execution_mode == ExecutionMode::Dynamic {
            if let Some(ref selected) = response.selected_tool {
                // Get the tool schema
                let schema_response = self
                    .router
                    .get_method_schema(&selected.mcp_server, &selected.tool_name)
                    .await
                    .map_err(|err| err.to_string())?;

                if let Some(schema) = schema_response.schema {
                    // Register the tool dynamically
                    let description = schema_response
                        .description
                        .unwrap_or_else(|| selected.rationale.clone());

                    let tool_definition = Self::build_dynamic_tool_definition(
                        &selected.tool_name,
                        &description,
                        schema.clone(),
                    );

                    let is_new = self
                        .tool_registry
                        .register_proxied_tool(
                            selected.mcp_server.clone(),
                            selected.tool_name.clone(),
                            tool_definition,
                        )
                        .await
                        .map_err(|err| err.to_string())?;

                    // Send notification if this is a new tool
                    if is_new {
                        eprintln!("📝 Dynamically registered tool: {}", selected.tool_name);

                        // Send ToolListChangedNotification to client
                        if self.peer.read().await.is_some() {
                            // Note: Notification sending disabled due to rmcp API constraints
                            // The client should re-query tools after receiving intelligent_route response
                            eprintln!(
                                "   📝 Tool '{}' registered - client should re-query tool list",
                                selected.tool_name
                            );
                        }
                    }

                    response.tool_schema = Some(schema);
                    response.dynamically_registered = true;
                    response.message = format!(
                        "Tool '{}' registered. Call it directly with full context for accurate parameters.",
                        selected.tool_name
                    );
                }
            }
        }

        Ok(Json(response))
    }

    #[tool(
        name = "start_task",
        description = "Launch an AI CLI task in background. Returns a UUID task_id for tracking. Options: role (inject prompt), provider (select API provider), cwd (working directory), cli_args (pass-through CLI arguments), worktree (git worktree isolation)."
    )]
    pub async fn start_task_tool(
        &self,
        params: Parameters<StartTaskParams>,
    ) -> Result<Json<TaskLaunchInfo>, String> {
        start_task(params.0).await.map(Json)
    }

    #[tool(
        name = "list_tasks",
        description = "List all tracked MCP tasks (running and completed). Returns task_id, status, worktree_info for each task."
    )]
    pub async fn list_tasks_tool(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<Vec<TaskInfo>>, String> {
        list_tasks().await.map(Json)
    }

    #[tool(
        name = "stop_task",
        description = "Stop a running MCP task by task_id. Sends SIGTERM, waits 5s, then SIGKILL if needed."
    )]
    pub async fn stop_task_tool(
        &self,
        params: Parameters<StopTaskParams>,
    ) -> Result<Json<StopTaskResult>, String> {
        stop_task(params.0).await.map(Json)
    }

    #[tool(
        name = "get_task_logs",
        description = "Retrieve log content of a tracked task by task_id. Supports tail mode to get last N lines."
    )]
    pub async fn get_task_logs_tool(
        &self,
        params: Parameters<GetTaskLogsParams>,
    ) -> Result<Json<TaskLogsResult>, String> {
        get_task_logs(params.0).await.map(Json)
    }

    #[tool(
        name = "get_task_status",
        description = "Get detailed status of a task by task_id. Returns status, process_alive, exit_code, result, worktree_info, timing info."
    )]
    pub async fn get_task_status_tool(
        &self,
        params: Parameters<GetTaskStatusParams>,
    ) -> Result<Json<TaskStatusResult>, String> {
        get_task_status(params.0).await.map(Json)
    }

    #[tool(
        name = "list_roles",
        description = "List all available roles (builtin + user-defined from ~/.aiw/role/). Roles inject system prompts into AI CLI tasks."
    )]
    pub async fn list_roles_tool(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<ListRolesResult>, String> {
        list_roles().await.map(Json)
    }

    #[tool(
        name = "list_providers",
        description = "List all configured AI providers with their scenarios and compatibility. Shows default provider and which AI types each provider supports."
    )]
    pub async fn list_providers_tool(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<ListProvidersResult>, String> {
        list_providers().await.map(Json)
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🚀 Agentic-Warden intelligent MCP router ready (stdio transport)");
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        self.serve(transport).await?.waiting().await?;
        Ok(())
    }
}

impl ServerHandler for AgenticWardenMcpServer {
    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        let tools_snapshot = self.tool_registry.get_all_tool_definitions().await;
        let tools = (*tools_snapshot).clone();

        Ok(rmcp::model::ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        // First, try to call base tools via tool_router
        if self.tool_router.has_route(&request.name) {
            // This is a base tool, delegate to tool_router
            let tool_context = ToolCallContext::new(self, request, context);
            return self.tool_router.call(tool_context).await;
        }

        // Check if this is a dynamically registered tool
        if let Some(registered) = self.tool_registry.get_tool(&request.name).await {
            match registered {
                RegisteredTool::ProxiedMcp(proxy) => {
                    let result = self
                        .router
                        .execute_tool(crate::mcp_routing::models::ExecuteToolRequest {
                            mcp_server: proxy.server.clone(),
                            tool_name: proxy.original_name.clone(),
                            arguments: serde_json::Value::Object(
                                request.arguments.unwrap_or_default(),
                            ),
                            session_id: None,
                        })
                        .await
                        .map_err(|e| {
                            rmcp::ErrorData::internal_error(
                                format!("Tool execution failed: {}", e),
                                None,
                            )
                        })?;

                    if result.success {
                        self.tool_registry.record_execution(&request.name).await;
                        let content_str = result
                            .result
                            .as_ref()
                            .map(|r| serde_json::to_string_pretty(&r.output).unwrap_or_default())
                            .unwrap_or_default();
                        let structured = result.result.map(|r| r.output);

                        Ok(rmcp::model::CallToolResult {
                            content: vec![rmcp::model::Content::text(content_str)],
                            structured_content: structured,
                            is_error: None,
                            meta: None,
                        })
                    } else {
                        Err(rmcp::ErrorData::internal_error(result.message, None))
                    }
                }
                RegisteredTool::JsOrchestrated(js_tool) => {
                    let input = serde_json::Value::Object(request.arguments.unwrap_or_default());
                    let execution = self
                        .js_executor
                        .execute(&js_tool, input)
                        .await
                        .map_err(|err| Self::map_js_tool_error(err))?;

                    self.tool_registry.record_execution(&request.name).await;
                    eprintln!(
                        "⚙️  JS workflow '{}' completed in {} ms",
                        request.name, execution.duration_ms
                    );

                    let output_str = serde_json::to_string_pretty(&execution.output)
                        .unwrap_or_else(|_| execution.output.to_string());

                    Ok(rmcp::model::CallToolResult {
                        content: vec![rmcp::model::Content::text(output_str)],
                        structured_content: Some(execution.output),
                        is_error: None,
                        meta: None,
                    })
                }
            }
        } else {
            // Tool not found in either base or dynamic tools
            Err(rmcp::ErrorData::method_not_found::<
                rmcp::model::CallToolRequestMethod,
            >())
        }
    }

    async fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, rmcp::ErrorData> {
        // Create initial capabilities (before testing)
        let capabilities = ClientCapabilities::from_init_request(&request);

        eprintln!("🔌 MCP Client connected:");
        eprintln!("   Name: {}", capabilities.client_name);
        eprintln!("   Version: {}", capabilities.client_version);
        eprintln!("   Testing dynamic tools support...");

        // Clone Arc for background task
        let client_capabilities = Arc::clone(&self.client_capabilities);
        let peer = context.peer.clone();

        // Spawn background task to test dynamic tools support
        // We delay a bit to allow the MCP initialization handshake to complete
        tokio::spawn(async move {
            // Wait for initialization to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Test if client supports dynamic tool registration
            let supports = ClientCapabilities::test_dynamic_tools_support(&peer).await;

            // Update capabilities
            if let Some(caps) = client_capabilities.write().await.as_mut() {
                caps.supports_dynamic_tools = supports;

                eprintln!(
                    "   Mode: {}",
                    if supports {
                        "✅ Dynamic registration (primary mode)"
                    } else {
                        "⚠️  Two-phase negotiation (fallback mode)"
                    }
                );
            }
        });

        // Save initial capabilities
        *self.client_capabilities.write().await = Some(capabilities);

        // Save peer for sending notifications later
        *self.peer.write().await = Some(context.peer.clone());

        // Return server info and capabilities
        Ok(InitializeResult {
            protocol_version: request.protocol_version,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            server_info: Implementation {
                name: "agentic-warden".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Agentic Warden MCP Server".to_string()),
                icons: None,
                website_url: Some("https://github.com/putao520/agentic-warden".to_string()),
            },
            instructions: None,
        })
    }
}

impl AgenticWardenMcpServer {
    fn map_js_tool_error(err: Error) -> rmcp::ErrorData {
        let message = err.to_string();
        let lowered = message.to_ascii_lowercase();
        let prefix = if lowered.contains("timed out") {
            "JS workflow timed out"
        } else if lowered.contains("syntax") {
            "JS workflow syntax error"
        } else {
            "JS workflow execution failed"
        };

        rmcp::ErrorData::internal_error(format!("{prefix}: {message}"), None)
    }
}
