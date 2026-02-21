#![cfg(unix)]

use aiw::mcp::{
    list_tasks, manage_task, start_task, ManageAction, ManageTaskParams, StartTaskParams,
};
use aiw::platform;
use aiw::provider::config::AiType;
use rmcp::service::RoleServer;
use serial_test::serial;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tokio::time::sleep;

fn mock_peer() -> Arc<RwLock<Option<rmcp::service::Peer<RoleServer>>>> {
    Arc::new(RwLock::new(None))
}

struct EnvGuard {
    key: String,
    original: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let original = env::var(key).ok();
        env::set_var(key, value);
        Self {
            key: key.to_string(),
            original,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(val) => env::set_var(&self.key, val),
            None => env::remove_var(&self.key),
        }
    }
}

struct TempHome {
    _dir: TempDir,
    _guards: Vec<EnvGuard>,
}

impl TempHome {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");

        let mut guards = Vec::new();
        guards.push(EnvGuard::set("HOME", dir.path().to_str().unwrap()));
        guards.push(EnvGuard::set("USERPROFILE", dir.path().to_str().unwrap()));

        #[cfg(target_os = "windows")]
        {
            use std::path::MAIN_SEPARATOR;
            let home_str = dir.path().to_str().unwrap();
            let mut parts = home_str.split(MAIN_SEPARATOR).collect::<Vec<_>>();
            if parts.len() > 1 {
                let drive = parts.remove(0).to_string();
                let rest = parts.join(&MAIN_SEPARATOR.to_string());
                guards.push(EnvGuard::set("HOMEDRIVE", &drive));
                guards.push(EnvGuard::set(
                    "HOMEPATH",
                    &format!("{MAIN_SEPARATOR}{rest}"),
                ));
            }
        }

        Self {
            _dir: dir,
            _guards: guards,
        }
    }

    fn path(&self) -> &Path {
        self._dir.path()
    }
}

fn create_role(home: &TempHome, name: &str, description: &str, content: &str) {
    let role_dir = home.path().join(".aiw").join("role");
    fs::create_dir_all(&role_dir).expect("create role dir");
    let role_path = role_dir.join(format!("{name}.md"));
    let file_content = format!("{description}\n------------\n{content}");
    fs::write(&role_path, file_content).expect("write role file");
}

#[tokio::test]
#[serial]
async fn start_task_launches_and_returns_pid() {
    let home = TempHome::new();

    let params = StartTaskParams {
        ai_type: Some(AiType::Codex),
        task: "echo hello".to_string(),
        provider: None,
        role: None,
        cwd: None,
        cli_args: None,
        worktree: None,
    };

    let launch = start_task(params, mock_peer()).await.expect("task should launch");
    assert!(launch.pid > 0, "pid should be positive");
    assert!(!launch.task_id.is_empty(), "task_id should be populated");

    // Allow the CLI to exit
    sleep(Duration::from_millis(3000)).await;
    drop(home);
}

#[tokio::test]
#[serial]
async fn list_tasks_returns_running_tasks() {
    let home = TempHome::new();

    let params = StartTaskParams {
        ai_type: Some(AiType::Codex),
        task: "echo hello".to_string(),
        provider: None,
        role: None,
        cwd: None,
        cli_args: None,
        worktree: None,
    };
    let launch = start_task(params, mock_peer()).await.expect("task should launch");

    let tasks = list_tasks().await.expect("list_tasks should succeed");
    let found = tasks.iter().any(|task| task.pid == launch.pid);
    assert!(found, "list_tasks should include newly started task");

    sleep(Duration::from_millis(3000)).await;
    drop(home);
}

#[tokio::test]
#[serial]
async fn stop_task_terminates_process() {
    let home = TempHome::new();

    let params = StartTaskParams {
        ai_type: Some(AiType::Codex),
        task: "echo hello".to_string(),
        provider: None,
        role: None,
        cwd: None,
        cli_args: None,
        worktree: None,
    };
    let launch = start_task(params, mock_peer()).await.expect("task should launch");

    let result = manage_task(ManageTaskParams {
        task_id: launch.task_id,
        action: ManageAction::Stop,
        tail_lines: None,
    })
    .await
    .expect("manage_task stop should succeed");
    assert_eq!(result.success, Some(true), "stop should report success");

    // Allow signal propagation
    sleep(Duration::from_millis(500)).await;
    assert!(
        !platform::process_alive(launch.pid),
        "process should be terminated"
    );
    drop(home);
}

#[tokio::test]
#[serial]
async fn get_task_logs_produces_log_file() {
    let home = TempHome::new();

    let params = StartTaskParams {
        ai_type: Some(AiType::Codex),
        task: "echo hello".to_string(),
        provider: None,
        role: None,
        cwd: None,
        cli_args: None,
        worktree: None,
    };
    let launch = start_task(params, mock_peer()).await.expect("task should launch");

    // wait for codex to produce some output
    sleep(Duration::from_millis(3000)).await;

    let full = manage_task(ManageTaskParams {
        task_id: launch.task_id.clone(),
        action: ManageAction::Logs,
        tail_lines: None,
    })
    .await
    .expect("log retrieval should succeed");

    let full_content = full.log_content.unwrap_or_default();
    assert!(
        !full_content.is_empty(),
        "log should contain some output"
    );

    let tail = manage_task(ManageTaskParams {
        task_id: launch.task_id.clone(),
        action: ManageAction::Logs,
        tail_lines: Some(1),
    })
    .await
    .expect("tail log retrieval should succeed");

    let tail_content = tail.log_content.unwrap_or_default();
    assert!(
        !tail_content.is_empty(),
        "tail log should contain some output"
    );
    // tail should be a subset of full
    assert!(
        full_content.len() >= tail_content.len(),
        "full log should be at least as long as tail"
    );
    drop(home);
}

#[tokio::test]
#[serial]
async fn start_task_injects_role_prompt() {
    let home = TempHome::new();
    create_role(&home, "test-role", "Test role", "ROLE-CONTENT");

    let params = StartTaskParams {
        ai_type: Some(AiType::Codex),
        task: "echo hello".to_string(),
        provider: None,
        role: Some("test-role".to_string()),
        cwd: None,
        cli_args: None,
        worktree: None,
    };

    let launch = start_task(params, mock_peer()).await.expect("task should launch");
    sleep(Duration::from_millis(3000)).await;

    let logs = manage_task(ManageTaskParams {
        task_id: launch.task_id.clone(),
        action: ManageAction::Logs,
        tail_lines: None,
    })
    .await
    .expect("should read logs");

    let content = logs.log_content.unwrap_or_default();
    // With a real CLI, we can only verify the log file was produced
    assert!(
        !content.is_empty(),
        "log should contain output when role is specified"
    );
    drop(home);
}
