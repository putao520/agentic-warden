use agentic_warden::core::models::{AiCliProcessInfo, ProcessTreeInfo};
use agentic_warden::core::process_tree;
use agentic_warden::memory::{ConversationHistoryStore, ConversationRecord};
use agentic_warden::mcp::TaskInfo;
use agentic_warden::storage::{RegistryEntry, SharedMemoryStorage};
use agentic_warden::task_record::{TaskRecord, TaskStatus};
use agentic_warden::unified_registry::Registry;
use anyhow::Result;
use chrono::Utc;
use once_cell::sync::Lazy;
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use tempfile::NamedTempFile;
use tokio::fs;

pub type ProcessTree = ProcessTreeInfo;
pub type RootAiCli = AiCliProcessInfo;

#[derive(Debug, Clone)]
pub struct SharedMemoryInfo {
    pub namespace: String,
    pub entries: Vec<RegistryEntry>,
}

#[derive(Debug, Clone)]
struct TrackedProcess {
    ai_type: String,
    task: String,
    env: HashMap<String, String>,
    tree: ProcessTreeInfo,
    log_path: PathBuf,
}

static TRACKED: Lazy<Mutex<HashMap<u32, TrackedProcess>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static CONVERSATION_DB_PATH: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));
static CLAUDE_LOG_PATH: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

#[derive(Debug, Clone)]
pub struct ProcessTreeSummary {
    pub groups: Vec<ProcessTreeInfo>,
}

fn sleep_command() -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "ping -n 3 127.0.0.1 > NUL"]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 2"]);
        cmd
    }
}

fn derive_namespace(pid: u32) -> String {
    format!("{}_task", pid)
}

pub fn set_process_env(pid: u32, env: HashMap<String, String>) {
    let mut guard = TRACKED.lock().expect("tracked map poisoned");
    if let Some(tracked) = guard.get_mut(&pid) {
        tracked.env = env;
    }
}

pub async fn spawn_ai_cli(ai_type: &str, task: &str) -> Result<TaskInfo> {
    let mut command = sleep_command();
    let mut child = command
        .spawn()
        .map_err(|err| anyhow::anyhow!("failed to spawn mock ai cli: {err}"))?;

    let pid = child.id();

    // Prevent zombie processes by waiting in background
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    let tree = process_tree::get_process_tree(pid)
        .unwrap_or_else(|_| ProcessTreeInfo::new(vec![pid, std::process::id()]));

    let log = NamedTempFile::new()
        .map_err(|err| anyhow::anyhow!("failed to create temp log: {err}"))?;
    let log_path = log.into_temp_path();
    let log_path_string = log_path.to_string_lossy().to_string();

    let record = TaskRecord::new(
        Utc::now(),
        format!("{ai_type}-{pid}"),
        log_path_string.clone(),
        Some(std::process::id()),
    )
    .with_process_tree_info(tree.clone())
    .unwrap_or_else(|_| {
        TaskRecord::new(
            Utc::now(),
            format!("{ai_type}-{pid}"),
            log_path_string.clone(),
            Some(std::process::id()),
        )
    });

    let registry = Registry::new(
        SharedMemoryStorage::connect_for_pid(pid)
            .map_err(|err| anyhow::anyhow!("failed to connect shared memory: {err}"))?,
    );
    registry
        .register(pid, &record)
        .map_err(|err| anyhow::anyhow!("failed to register task: {err}"))?;

    let mut guard = TRACKED.lock().expect("tracked map poisoned");
    guard.insert(
        pid,
        TrackedProcess {
            ai_type: ai_type.to_string(),
            task: task.to_string(),
            env: HashMap::new(),
            tree: tree.clone(),
            log_path: PathBuf::from(record.log_path.clone()),
        },
    );

    Ok(TaskInfo {
        pid,
        log_file: record.log_path,
        status: TaskStatus::Running,
        started_at: record.started_at,
        completed_at: None,
        cleanup_reason: None,
        manager_pid: record.manager_pid,
        exit_code: None,
        log_id: record.log_id,
    })
}

pub async fn get_process_tree() -> Result<ProcessTreeSummary> {
    let guard = TRACKED.lock().expect("tracked map poisoned");
    let groups = guard
        .values()
        .map(|tracked| tracked.tree.clone())
        .collect();
    Ok(ProcessTreeSummary { groups })
}

pub async fn detect_root_ai_cli(pid: i32) -> Option<RootAiCli> {
    let pid = pid.max(0) as u32;
    let guard = TRACKED.lock().ok()?;
    if let Some(tracked) = guard.get(&pid) {
        return Some(
            AiCliProcessInfo::new(pid, tracked.ai_type.clone())
                .with_process_name(format!("{}-mock", tracked.ai_type)),
        );
    }

    // try to find matching ancestor
    if let Ok(tree) = process_tree::get_process_tree(pid) {
        for ancestor in tree.process_chain {
            if let Some(entry) = guard.get(&ancestor) {
                return Some(
                    AiCliProcessInfo::new(ancestor, entry.ai_type.clone())
                        .with_process_name(format!("{}-mock", entry.ai_type)),
                );
            }
        }
    }
    None
}

pub async fn get_shared_memory(pid: i32) -> SharedMemoryInfo {
    let pid_u32 = pid.max(0) as u32;
    let namespace = derive_namespace(pid_u32);
    let storage = SharedMemoryStorage::connect_for_pid(pid_u32);
    let entries = storage
        .ok()
        .map(|store| Registry::new(store).entries().unwrap_or_default())
        .unwrap_or_default();
    SharedMemoryInfo { namespace, entries }
}

pub async fn get_process_env_vars(pid: i32) -> HashMap<String, String> {
    let pid = pid.max(0) as u32;
    TRACKED
        .lock()
        .map(|guard| guard.get(&pid).map(|t| t.env.clone()).unwrap_or_default())
        .unwrap_or_default()
}

pub async fn create_test_transcript(
    session_id: &str,
    messages: Vec<(&str, &str)>,
) -> Result<PathBuf> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(format!("{session_id}.jsonl"));
    let mut lines = Vec::new();
    for (role, content) in messages {
        let obj = serde_json::json!({ "role": role, "content": content });
        lines.push(obj.to_string());
    }
    fs::write(&path, lines.join("\n")).await?;
    // Keep directory alive by moving it into global store
    {
        let mut guard = CONVERSATION_DB_PATH.lock().unwrap();
        if guard.is_none() {
            *guard = Some(dir.into_path().join("conversation_history.db"));
        } else {
            // prevent dir dropping early
            let _ = dir.into_path();
        }
    }
    Ok(path)
}

fn get_conversation_db_path() -> PathBuf {
    let mut guard = CONVERSATION_DB_PATH.lock().unwrap();
    if let Some(path) = guard.as_ref() {
        return path.clone();
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("conversation_history.db");
    *guard = Some(path.clone());
    let _ = dir.into_path();
    path
}

fn embed_text(text: &str, dimension: usize) -> Vec<f32> {
    let mut vec = vec![0.0; dimension];
    for (i, byte) in text.bytes().enumerate() {
        let idx = i % dimension;
        vec[idx] += (byte as f32) / 255.0;
    }
    vec
}

pub fn simple_embed(text: &str) -> Vec<f32> {
    embed_text(text, 4)
}

pub fn reset_conversation_store() -> Result<()> {
    let mut guard = CONVERSATION_DB_PATH.lock().unwrap();
    if let Some(path) = guard.take() {
        let _ = std::fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }
    Ok(())
}

pub fn conversation_store() -> Result<ConversationHistoryStore> {
    let path = get_conversation_db_path();
    ConversationHistoryStore::new(&path, 4).map_err(|err| anyhow::anyhow!(err))
}

pub async fn setup_test_conversations(conversations: Vec<(&str, &str)>) -> Result<()> {
    let db_path = get_conversation_db_path();
    let store = ConversationHistoryStore::new(&db_path, 4)?;

    for (session, content) in conversations {
        let record = ConversationRecord::new(Some(session.to_string()), "assistant", content, vec![]);
        let embedding = embed_text(content, 4);
        store.append(record, embedding)?;
    }
    Ok(())
}

pub async fn process_hook_stdin(input: Value) -> Result<()> {
    let session_id = input
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let transcript_path = input
        .get("transcript_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("transcript_path required"))?;

    let data = fs::read_to_string(&transcript_path).await?;
    let mut records = Vec::new();
    for line in data.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: Value = serde_json::from_str(line)?;
        let role = parsed
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("assistant")
            .to_string();
        let content = parsed
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        records.push((role, content));
    }

    let db_path = get_conversation_db_path();
    let store = ConversationHistoryStore::new(&db_path, 4)?;
    for (role, content) in records {
        let record = ConversationRecord::new(Some(session_id.to_string()), role, content.clone(), vec![]);
        let embedding = embed_text(&content, 4);
        store.append(record, embedding)?;
    }
    Ok(())
}

pub async fn setup_test_provider(name: &str) -> Result<()> {
    use agentic_warden::provider::config::{Provider, ProvidersConfig};

    let temp_home = tempfile::tempdir()?;
    let home_path = temp_home.into_path();

    std::env::set_var("HOME", &home_path);
    std::env::set_var("USERPROFILE", &home_path);

    let mut providers = HashMap::new();
    providers.insert(
        name.to_string(),
        Provider {
            token: Some("sk-test-123".into()),
            base_url: Some("https://openrouter.ai/api/v1".into()),
            scenario: None,
            env: HashMap::from([("CUSTOM_KEY".into(), "custom_value".into())]),
        },
    );

    let config = ProvidersConfig {
        schema: None,
        providers,
        default_provider: name.to_string(),
        memory: None,
    };

    let config_dir = home_path.join(".aiw");
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("providers.json");
    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&config)
            .map_err(|err| anyhow::anyhow!("failed to serialize provider config: {err}"))?,
    )?;

    Ok(())
}

pub async fn create_mock_npm_bin() -> Result<PathBuf> {
    let dir = tempfile::tempdir()?;
    let bin_path = dir.path().join(if cfg!(windows) { "npm.cmd" } else { "npm" });
    let log_path = dir.path().join("npm_mock.log");

    if cfg!(windows) {
        let script = format!(
            "@echo off\r\necho npm %* >> \"{}\"\r\nexit /B 0",
            log_path.display()
        );
        std::fs::write(&bin_path, script)?;
    } else {
        let script = format!(
            "#!/usr/bin/env sh\n\necho npm \"$@\" >> \"{}\"\nexit 0\n",
            log_path.display()
        );
        std::fs::write(&bin_path, script)?;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bin_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin_path, perms)?;
    }

    // prevent temp dir cleanup
    let _ = dir.into_path();
    Ok(bin_path)
}

pub async fn create_mock_claude_bin() -> Result<PathBuf> {
    let dir = tempfile::tempdir()?;
    let bin_path = dir
        .path()
        .join(if cfg!(windows) { "claude.cmd" } else { "claude" });
    let log_path = dir.path().join("claude_mock.log");

    {
        let mut guard = CLAUDE_LOG_PATH.lock().unwrap();
        *guard = Some(log_path.clone());
    }

    if cfg!(windows) {
        let script = format!(
            "@echo off\r\necho claude %* >> \"{}\"\r\nexit /B 0",
            log_path.display()
        );
        std::fs::write(&bin_path, script)?;
    } else {
        let script = format!(
            "#!/usr/bin/env sh\n\necho claude \"$@\" >> \"{}\"\nexit 0\n",
            log_path.display()
        );
        std::fs::write(&bin_path, script)?;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bin_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin_path, perms)?;
    }

    let _ = dir.into_path();
    Ok(bin_path)
}

pub async fn read_mock_claude_log() -> Result<String> {
    let path = CLAUDE_LOG_PATH
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("mock claude log not initialized"))?;
    let content = fs::read_to_string(path).await?;
    Ok(content)
}

pub struct McpServerTestHarness {
    tools: Mutex<Vec<Tool>>,
}

impl McpServerTestHarness {
    pub fn new() -> Self {
        let mut tools = Vec::new();
        let empty_schema: serde_json::Map<String, Value> = serde_json::Map::new();
        tools.push(Tool::new(
            "intelligent_route",
            "Route to best MCP tool",
            empty_schema.clone(),
        ));
        tools.push(Tool::new(
            "search_history",
            "Search conversation history",
            empty_schema,
        ));
        Self {
            tools: Mutex::new(tools),
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        Ok(self.tools.lock().unwrap().clone())
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        match name {
            "intelligent_route" => {
                let user_request = args
                    .get("user_request")
                    .and_then(Value::as_str)
                    .unwrap_or("dynamic_tool");
                let tool_name = format!(
                    "{}_tool",
                    user_request.replace(' ', "_").to_lowercase()
                );

                let mut registry = self.tools.lock().unwrap();
                let new_tool = Tool::new(
                    tool_name.clone(),
                    format!("dynamic tool for {user_request}"),
                    serde_json::Map::new(),
                );
                registry.push(new_tool);

                Ok(serde_json::json!({
                    "selected_tool": { "tool_name": tool_name },
                    "dynamically_registered": true
                }))
            }
            other => {
                let registry = self.tools.lock().unwrap();
                if registry.iter().any(|t| t.name == other) {
                    Ok(serde_json::json!({ "ok": true, "tool": other }))
                } else if other == "search_history" {
                    Ok(serde_json::json!({ "results": Vec::<Value>::new() }))
                } else {
                    Err(anyhow::anyhow!("tool not found: {other}"))
                }
            }
        }
    }
}

pub async fn start_mcp_server() -> McpServerTestHarness {
    McpServerTestHarness::new()
}
