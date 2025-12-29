#![cfg(unix)]

use aiw::mcp::{
    get_task_logs, list_tasks, start_task, stop_task, GetTaskLogsParams, StartTaskParams,
    StopTaskParams,
};
use aiw::platform;
use aiw::task_record::TaskStatus;
use serial_test::serial;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

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

fn make_test_cli(root: &TempDir, name: &str, sleep_secs: f32, extra_lines: &[&str]) -> PathBuf {
    let path = root.path().join(name);
    let mut script = String::from("#!/bin/sh\n");

    for line in extra_lines {
        script.push_str(&format!("echo {}\n", line));
    }

    script.push_str("echo \"$@\"\n");
    script.push_str(&format!("sleep {}\n", sleep_secs));

    fs::write(&path, script).expect("write cli stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("set permissions");
    }
    path
}

fn create_role(home: &TempHome, name: &str, description: &str, content: &str) -> PathBuf {
    let role_dir = home.path().join(".aiw").join("role");
    fs::create_dir_all(&role_dir).expect("create role dir");
    let role_path = role_dir.join(format!("{name}.md"));
    let file_content = format!("{description}\n------------\n{content}");
    fs::write(&role_path, file_content).expect("write role file");
    role_path
}

#[tokio::test]
#[serial]
async fn start_task_launches_and_returns_pid() {
    let home = TempHome::new();
    let cli_root = TempDir::new().expect("cli root");
    let cli_path = make_test_cli(&cli_root, "cli-start.sh", 1.0, &["start-task"]);
    let _bin_guard = EnvGuard::set("CODEX_BIN", cli_path.to_str().unwrap());

    let params = StartTaskParams {
        ai_type: "codex".to_string(),
        task: "run-start-test".to_string(),
        provider: None,
        role: None,
        cwd: None,
    };

    let launch = start_task(params).await.expect("task should launch");
    assert!(launch.pid > 0, "pid should be positive");
    assert_eq!(launch.status, TaskStatus::Running);
    assert!(!launch.log_file.is_empty(), "log file should be populated");
    assert!(
        Path::new(&launch.log_file).exists(),
        "log file path should exist"
    );

    // Allow the stub CLI to exit to avoid leak
    sleep(Duration::from_millis(1500)).await;
    drop(home);
}

#[tokio::test]
#[serial]
async fn list_tasks_returns_running_tasks() {
    let home = TempHome::new();
    let cli_root = TempDir::new().expect("cli root");
    let cli_path = make_test_cli(&cli_root, "cli-list.sh", 2.0, &["list-task"]);
    let _bin_guard = EnvGuard::set("CODEX_BIN", cli_path.to_str().unwrap());

    let params = StartTaskParams {
        ai_type: "codex".to_string(),
        task: "list-task".to_string(),
        provider: None,
        role: None,
        cwd: None,
    };
    let launch = start_task(params).await.expect("task should launch");

    let tasks = list_tasks().await.expect("list_tasks should succeed");
    let found = tasks.iter().any(|task| task.pid == launch.pid);
    assert!(found, "list_tasks should include newly started task");

    sleep(Duration::from_millis(2100)).await;
    drop(home);
}

#[tokio::test]
#[serial]
async fn stop_task_terminates_process() {
    let home = TempHome::new();
    let cli_root = TempDir::new().expect("cli root");
    let cli_path = make_test_cli(&cli_root, "cli-stop.sh", 10.0, &["stop-target"]);
    let _bin_guard = EnvGuard::set("CODEX_BIN", cli_path.to_str().unwrap());

    let params = StartTaskParams {
        ai_type: "codex".to_string(),
        task: "stop-me".to_string(),
        provider: None,
        role: None,
        cwd: None,
    };
    let launch = start_task(params).await.expect("task should launch");

    let result = stop_task(StopTaskParams { pid: launch.pid })
        .await
        .expect("stop_task should succeed");
    assert!(result.success, "stop_task should report success");

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
async fn get_task_logs_supports_full_and_tail_modes() {
    let home = TempHome::new();
    let cli_root = TempDir::new().expect("cli root");
    let cli_path = make_test_cli(
        &cli_root,
        "cli-logs.sh",
        0.2,
        &["alpha-line", "beta-line", "gamma-line"],
    );
    let _bin_guard = EnvGuard::set("CODEX_BIN", cli_path.to_str().unwrap());

    let params = StartTaskParams {
        ai_type: "codex".to_string(),
        task: "log-test".to_string(),
        provider: None,
        role: None,
        cwd: None,
    };
    let launch = start_task(params).await.expect("task should launch");

    // wait for log writing to finish
    sleep(Duration::from_millis(400)).await;

    let full = get_task_logs(GetTaskLogsParams {
        pid: launch.pid,
        tail_lines: None,
    })
    .await
    .expect("log retrieval should succeed");

    assert!(
        full.content.contains("alpha-line") && full.content.contains("gamma-line"),
        "full log should contain all lines"
    );

    let tail = get_task_logs(GetTaskLogsParams {
        pid: launch.pid,
        tail_lines: Some(1),
    })
    .await
    .expect("tail log retrieval should succeed");

    assert!(
        tail.content.trim_end().contains("log-test") && !tail.content.contains("alpha-line"),
        "tail mode should return the final log line content"
    );
    drop(home);
}

#[tokio::test]
#[serial]
async fn start_task_injects_role_prompt() {
    let home = TempHome::new();
    create_role(&home, "test-role", "Test role", "ROLE-CONTENT");

    let cli_root = TempDir::new().expect("cli root");
    let cli_path = make_test_cli(&cli_root, "cli-role.sh", 0.1, &[]);
    let _bin_guard = EnvGuard::set("CODEX_BIN", cli_path.to_str().unwrap());

    let params = StartTaskParams {
        ai_type: "codex".to_string(),
        task: "user-task".to_string(),
        provider: None,
        role: Some("test-role".to_string()),
        cwd: None,
    };

    let launch = start_task(params).await.expect("task should launch");
    sleep(Duration::from_millis(300)).await;

    let logs = get_task_logs(GetTaskLogsParams {
        pid: launch.pid,
        tail_lines: None,
    })
    .await
    .expect("should read logs");

    assert!(
        logs.content.contains("ROLE-CONTENT"),
        "role content should be injected into prompt"
    );
    assert!(
        logs.content.contains("user-task"),
        "user task should still be included"
    );
    drop(home);
}
