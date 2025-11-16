//! Interactive Mode E2E Tests
//! Tests REQ-009: 交互式AI CLI启动

mod common;

use agentic_warden::cli_type::CliType;
use agentic_warden::commands::ai_cli::AiCliCommand;
use agentic_warden::platform;
use agentic_warden::registry_factory::RegistryFactory;
use anyhow::Result;
use common::{create_mock_claude_bin, setup_test_provider};
use serial_test::serial;
use std::env;
use std::path::PathBuf;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tempfile;

#[derive(Debug)]
struct EnvGuard {
    key: String,
    original: Option<String>,
}

impl EnvGuard {
    fn set(key: impl Into<String>, value: impl Into<String>) -> Self {
        let key_string = key.into();
        let original = env::var(&key_string).ok();
        env::set_var(&key_string, value.into());
        Self { key: key_string, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(original) = &self.original {
            env::set_var(&self.key, original);
        } else {
            env::remove_var(&self.key);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_interactive_mode_with_provider() -> Result<()> {
    let home_guard = EnvGuard::set("HOME", tempfile::tempdir()?.into_path().display().to_string());
    let user_guard = EnvGuard::set("USERPROFILE", env::var("HOME").unwrap_or_default());
    setup_test_provider("openrouter").await?;

    let claude_bin = create_mock_claude_bin().await?;
    let bin_dir = claude_bin
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut path_entries: Vec<PathBuf> =
        env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect();
    path_entries.insert(0, bin_dir.clone());
    let joined_path = env::join_paths(path_entries)?;

    let _bin_guard = EnvGuard::set("CLAUDE_BIN", claude_bin.to_string_lossy().to_string());
    let _path_guard = EnvGuard::set("PATH", joined_path.to_string_lossy().to_string());

    let command = AiCliCommand::new(vec![CliType::Claude], Some("openrouter".into()), String::new());
    let exit_code = command.execute().await?;
    assert_eq!(exit_code, std::process::ExitCode::from(0));

    // Verify registry recorded the interactive session
    let registry = RegistryFactory::instance()
        .get_cli_registry()
        .expect("cli registry");
    let entries = registry.entries().unwrap_or_default();
    assert!(
        !entries.is_empty(),
        "interactive session should register in CLI registry"
    );

    drop(_path_guard);
    drop(_bin_guard);
    drop(user_guard);
    drop(home_guard);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_interactive_signal_handling() -> Result<()> {
    let mut cmd = if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", "ping -n 6 127.0.0.1 > NUL"]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 5"]);
        command
    };

    let mut child = cmd.spawn()?;
    let pid = child.id().expect("child pid");
    sleep(Duration::from_millis(300)).await;

    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        kill(Pid::from_raw(pid as i32), Signal::SIGINT)?;
    }

    #[cfg(windows)]
    {
        platform::terminate_process(pid as u32);
    }

    let status = tokio::time::timeout(Duration::from_secs(5), child.wait()).await??;
    assert!(
        !platform::process_alive(pid),
        "child process should be terminated gracefully, status: {status:?}"
    );

    Ok(())
}
