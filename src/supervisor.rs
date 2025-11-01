use crate::cli_type::CliType;
use crate::logging::debug;
use crate::logging::warn;
use crate::platform::{self, ChildResources};
use crate::process_tree::{ProcessTreeError, ProcessTreeInfo};
use crate::provider::{AiType, EnvInjector, ProviderManager};
use crate::registry::{RegistryError, TaskRegistry};
use crate::signal;
use crate::task_record::TaskRecord;
use chrono::{DateTime, Utc};
use std::env;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),
    #[error("Process tree error: {0}")]
    ProcessTree(#[from] ProcessTreeError),
    #[error("CLI executable not found: {0}")]
    CliNotFound(String),
    #[error("{0}")]
    Other(String),
}

fn get_cli_command(cli_type: &CliType) -> Result<String, ProcessError> {
    // First try environment variable
    if let Ok(custom_path) = env::var(cli_type.env_var_name()) {
        if custom_path.is_empty() {
            return Err(ProcessError::CliNotFound(format!(
                "{} environment variable is empty",
                cli_type.env_var_name()
            )));
        }
        return Ok(custom_path);
    }

    // Fall back to default command name
    let default_cmd = cli_type.command_name();

    // On Windows, try to find the actual executable path
    if cfg!(windows) {
        let output = Command::new("where")
            .arg(default_cmd)
            .output()
            .map_err(|_| {
                ProcessError::CliNotFound(format!(
                    "Failed to check if '{}' exists in PATH",
                    default_cmd
                ))
            })?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Prefer .cmd files on Windows, otherwise use first result
            for line in stdout.lines() {
                if line.ends_with(".cmd") || line.ends_with(".bat") || line.ends_with(".exe") {
                    return Ok(line.to_string());
                }
            }
            // Fallback to first line if no Windows executable found
            if let Some(first_line) = stdout.lines().next() {
                return Ok(first_line.to_string());
            }
        }

        return Err(ProcessError::CliNotFound(format!(
            "'{}' not found in PATH. Set {} environment variable or ensure it's in PATH",
            default_cmd,
            cli_type.env_var_name()
        )));
    } else {
        let output = Command::new("which")
            .arg(default_cmd)
            .output()
            .map_err(|_| {
                ProcessError::CliNotFound(format!(
                    "Failed to check if '{}' exists in PATH",
                    default_cmd
                ))
            })?;

        if !output.status.success() {
            return Err(ProcessError::CliNotFound(format!(
                "'{}' not found in PATH. Set {} environment variable or ensure it's in PATH",
                default_cmd,
                cli_type.env_var_name()
            )));
        }
    }

    Ok(default_cmd.to_string())
}

pub fn execute_cli(
    registry: &TaskRegistry,
    cli_type: &CliType,
    args: &[OsString],
    provider: Option<String>,
) -> Result<i32, ProcessError> {
    platform::init_platform();

    registry.sweep_stale_entries(
        Utc::now(),
        platform::process_alive,
        &platform::terminate_process,
    )?;

    // Load provider configuration
    let provider_manager = ProviderManager::new()
        .map_err(|e| ProcessError::Other(format!("Failed to load provider: {}", e)))?;

    // Determine which provider to use
    let (provider_name, provider_config) = if let Some(name) = provider {
        let config = provider_manager
            .get_provider(&name)
            .map_err(|e| ProcessError::Other(e.to_string()))?;
        (name, config)
    } else {
        let (name, config) = provider_manager
            .get_default_provider()
            .ok_or_else(|| ProcessError::Other("No default provider configured".to_string()))?;
        (name, config)
    };

    // Validate compatibility
    let ai_type = match cli_type {
        CliType::Claude => AiType::Claude,
        CliType::Codex => AiType::Codex,
        CliType::Gemini => AiType::Gemini,
    };

    provider_manager
        .validate_compatibility(&provider_name, ai_type)
        .map_err(|e| ProcessError::Other(e.to_string()))?;

    // Display provider info if not using official
    if provider_name != *"official" {
        eprintln!(
            "Using provider: {} ({})",
            provider_name, provider_config.description
        );
    }

    let cli_command = get_cli_command(cli_type)?;

    let mut command = Command::new(&cli_command);
    command.args(args);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    platform::prepare_command(&mut command)?;

    // Inject environment variables to child process
    EnvInjector::inject_to_command(&mut command, &provider_config.env);

    let mut child = command.spawn()?;
    let child_pid = child.id();

    let log_path = match generate_log_path(child_pid) {
        Ok(path) => path,
        Err(err) => {
            platform::terminate_process(child_pid);
            let _ = child.wait();
            return Err(err.into());
        }
    };

    let log_file = match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
    {
        Ok(file) => file,
        Err(err) => {
            platform::terminate_process(child_pid);
            let _ = child.wait();
            return Err(err.into());
        }
    };

    debug(format!(
        "Started Codex process pid={} log={}",
        child_pid,
        log_path.display()
    ));

    let _resources: ChildResources = platform::after_spawn(&child)?;
    let signal_guard = signal::install(child_pid)?;

    let log_writer = Arc::new(Mutex::new(BufWriter::new(log_file)));
    let mut copy_handles = Vec::new();

    if let Some(stdout) = child.stdout.take() {
        copy_handles.push(spawn_copy(stdout, log_writer.clone(), StreamMirror::Stdout));
    }
    if let Some(stderr) = child.stderr.take() {
        copy_handles.push(spawn_copy(stderr, log_writer.clone(), StreamMirror::Stderr));
    }

    let registration_guard = if true {
        let mut record = TaskRecord::new(
            Utc::now(),
            child_pid.to_string(),
            log_path.to_string_lossy().into_owned(),
            Some(platform::current_pid()),
        );

        // Get process tree information (core functionality)
        match ProcessTreeInfo::current() {
            Ok(tree_info) => {
                record = record.with_process_tree(
                    tree_info.process_chain,
                    tree_info.root_parent_pid,
                    tree_info.depth,
                );
            }
            Err(err) => {
                warn(format!("Failed to get process tree info: {}", err));
                // Continue with basic record creation
            }
        }

        if let Err(err) = registry.register(child_pid, &record) {
            platform::terminate_process(child_pid);
            let _ = child.wait();
            return Err(err.into());
        }
        Some(RegistrationGuard::new(registry, child_pid))
    } else {
        None
    };

    let status = child.wait()?;
    drop(signal_guard);

    for handle in copy_handles {
        match handle.join() {
            Ok(result) => result?,
            Err(_) => {
                return Err(io::Error::other("Log writer thread failed").into());
            }
        }
    }

    {
        let mut writer = log_writer
            .lock()
            .map_err(|_| io::Error::other("Log writer lock poisoned"))?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }

    // Display log file path to user
    eprintln!("完整日志已保存到: {}", log_path.display());

    if let Some(guard) = registration_guard {
        let completed_at = Utc::now();
        let exit_code = status.code();
        let result = match (status.success(), exit_code) {
            (true, _) => Some("success".to_owned()),
            (false, Some(code)) => Some(format!("failed_with_exit_code_{code}")),
            (false, None) => Some("failed_without_exit_code".to_owned()),
        };
        let _ = guard.mark_completed(result, exit_code, completed_at);
    }

    Ok(extract_exit_code(status))
}

fn generate_log_path(pid: u32) -> io::Result<PathBuf> {
    let tmp = std::env::temp_dir();
    Ok(tmp.join(format!("{pid}.log")))
}

#[derive(Copy, Clone)]
enum StreamMirror {
    Stdout,
    Stderr,
}

impl StreamMirror {
    fn write(self, data: &[u8]) -> io::Result<()> {
        match self {
            StreamMirror::Stdout => {
                let mut handle = io::stdout().lock();
                handle.write_all(data)?;
                handle.flush()
            }
            StreamMirror::Stderr => {
                let mut handle = io::stderr().lock();
                handle.write_all(data)?;
                handle.flush()
            }
        }
    }
}

fn spawn_copy<R>(
    mut reader: R,
    writer: Arc<Mutex<BufWriter<std::fs::File>>>,
    mirror: StreamMirror,
) -> thread::JoinHandle<io::Result<()>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let chunk = &buffer[..read];
            {
                let mut guard = writer
                    .lock()
                    .map_err(|_| io::Error::other("Log writer lock poisoned"))?;
                guard.write_all(chunk)?;
                guard.flush()?;
            }
            mirror.write(chunk)?;
        }
        Ok(())
    })
}

fn extract_exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

struct RegistrationGuard<'a> {
    registry: &'a TaskRegistry,
    pid: u32,
    active: bool,
}

impl<'a> RegistrationGuard<'a> {
    fn new(registry: &'a TaskRegistry, pid: u32) -> Self {
        Self {
            registry,
            pid,
            active: true,
        }
    }

    fn mark_completed(
        mut self,
        result: Option<String>,
        exit_code: Option<i32>,
        completed_at: DateTime<Utc>,
    ) -> Result<(), RegistryError> {
        if self.active {
            self.registry
                .mark_completed(self.pid, result, exit_code, completed_at)?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for RegistrationGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.registry.remove(self.pid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_record::TaskStatus;
    use std::io::Read;

    #[test]
    fn test_generate_log_path() {
        let test_pid = 12345;
        let path = generate_log_path(test_pid).unwrap();

        // Should be in temp directory with PID as filename
        assert!(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("12345")
        );
        assert_eq!(path.extension().unwrap().to_str().unwrap(), "log");

        // Parent should be temp directory
        assert_eq!(path.parent(), Some(&std::env::temp_dir()).map(|v| &**v));
    }

    #[test]
    fn test_stream_mirror_stdout() {
        let test_data = b"Hello, stdout!";
        StreamMirror::Stdout.write(test_data).unwrap();
        // Should write to stdout without error
    }

    #[test]
    fn test_stream_mirror_stderr() {
        let test_data = b"Hello, stderr!";
        StreamMirror::Stderr.write(test_data).unwrap();
        // Should write to stderr without error
    }

    #[test]
    fn test_extract_exit_code() {
        // Test with successful status
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            let success_status = ExitStatus::from_raw(0);
            assert_eq!(extract_exit_code(success_status), 0);

            let error_status = ExitStatus::from_raw(1);
            assert_eq!(extract_exit_code(error_status), 1);
        }

        #[cfg(windows)]
        {
            // Windows ExitStatus doesn't have from_raw, so we'll test the default case
            // In real scenarios, this would come from an actual process
        }
    }

    #[test]
    fn test_registration_guard_new_and_active() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_namespace = format!("test_guard_{}_{}", std::process::id(), timestamp);
        let registry = TaskRegistry::connect_test(&test_namespace).unwrap();
        let test_pid = 99999;

        let guard = RegistrationGuard::new(&registry, test_pid);
        assert!(guard.active);
        assert_eq!(guard.pid, test_pid);
    }

    #[test]
    fn test_registration_guard_mark_completed() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_namespace = format!("test_guard_complete_{}_{}", std::process::id(), timestamp);
        let registry = TaskRegistry::connect_test(&test_namespace).unwrap();
        let test_pid = 88888;

        // First register a task
        let record = TaskRecord::new(
            Utc::now(),
            test_pid.to_string(),
            format!("/tmp/{}.log", test_pid),
            Some(test_pid + 1000),
        );
        registry.register(test_pid, &record).unwrap();

        let guard = RegistrationGuard::new(&registry, test_pid);
        assert!(guard.active);

        // Mark as completed
        let result = Some("test completed".to_string());
        let exit_code = Some(0);
        let completed_at = Utc::now();

        // Mark as completed
        let was_active = guard.active;
        let _ = guard
            .mark_completed(result.clone(), exit_code, completed_at)
            .unwrap();

        // Guard should have been active before and now is consumed
        assert!(was_active);

        // Verify the record was updated
        let entries = registry.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].record.status, TaskStatus::CompletedButUnread);
        assert_eq!(entries[0].record.result, result);
        assert_eq!(entries[0].record.exit_code, exit_code);
    }

    #[test]
    fn test_registration_guard_drop_cleanup() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_namespace = format!("test_guard_drop_{}_{}", std::process::id(), timestamp);
        let registry = TaskRegistry::connect_test(&test_namespace).unwrap();
        let test_pid = 77777;

        // First register a task
        let record = TaskRecord::new(
            Utc::now(),
            test_pid.to_string(),
            format!("/tmp/{}.log", test_pid),
            Some(test_pid + 1000),
        );
        registry.register(test_pid, &record).unwrap();

        {
            let guard = RegistrationGuard::new(&registry, test_pid);
            assert!(guard.active);

            // Guard goes out of scope here without marking as completed
            // Should trigger cleanup (removal)
        }

        // Verify the record was removed
        let entries = registry.entries().unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_spawn_copy_basic_functionality() {
        use std::sync::mpsc;

        let (_tx, _rx) = mpsc::channel::<()>();
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let writer = Arc::new(Mutex::new(BufWriter::new(temp_file.reopen().unwrap())));

        // Create a simple reader that sends some data
        let data = b"Test data for copy";
        let reader_clone = data;

        let handle = spawn_copy(&reader_clone[..], writer.clone(), StreamMirror::Stdout);

        // Wait for the copy to complete
        handle.join().unwrap().unwrap();

        // Verify data was written to the file
        drop(writer); // Release the writer lock
        let mut file_content = String::new();
        temp_file
            .reopen()
            .unwrap()
            .read_to_string(&mut file_content)
            .unwrap();
        assert_eq!(file_content, "Test data for copy");
    }
}

/// 批量执行多个CLI任务
pub fn execute_multiple_clis(
    registry: &TaskRegistry,
    cli_selector: &crate::cli_type::CliSelector,
    prompt: &str,
    provider: Option<String>,
) -> Result<Vec<i32>, ProcessError> {
    let mut exit_codes = Vec::new();

    // 依次执行每个CLI
    for cli_type in &cli_selector.types {
        let args = cli_type.build_full_access_args(prompt);
        let os_args: Vec<OsString> = args.into_iter().map(|s| s.into()).collect();
        let exit_code = execute_cli(registry, cli_type, &os_args, provider.clone())?;
        exit_codes.push(exit_code);

        if exit_code == 0 {
            println!("{} task completed successfully!", cli_type.display_name());
        } else {
            println!(
                "{} task failed with exit code: {}",
                cli_type.display_name(),
                exit_code
            );
        }
    }

    Ok(exit_codes)
}

/// 启动AI CLI的交互模式（不执行任务，直接进入CLI交互界面）
pub fn start_interactive_cli(
    registry: &TaskRegistry,
    cli_type: &CliType,
    provider: Option<String>,
) -> Result<i32, ProcessError> {
    platform::init_platform();

    registry.sweep_stale_entries(
        Utc::now(),
        platform::process_alive,
        &platform::terminate_process,
    )?;

    // Load provider configuration
    let provider_manager = ProviderManager::new()
        .map_err(|e| ProcessError::Other(format!("Failed to load provider: {}", e)))?;

    // Determine which provider to use
    let (provider_name, provider_config) = if let Some(name) = provider {
        let config = provider_manager
            .get_provider(&name)
            .map_err(|e| ProcessError::Other(e.to_string()))?;
        (name, config)
    } else {
        let (name, config) = provider_manager
            .get_default_provider()
            .ok_or_else(|| ProcessError::Other("No default provider configured".to_string()))?;
        (name, config)
    };

    // Validate compatibility
    let ai_type = match cli_type {
        CliType::Claude => AiType::Claude,
        CliType::Codex => AiType::Codex,
        CliType::Gemini => AiType::Gemini,
    };

    provider_manager
        .validate_compatibility(&provider_name, ai_type)
        .map_err(|e| ProcessError::Other(e.to_string()))?;

    // Display provider info if not using official
    if provider_name != *"official" {
        eprintln!(
            "Using provider: {} ({})",
            provider_name, provider_config.description
        );
    }

    eprintln!(
        "🚀 Starting {} in interactive mode...",
        cli_type.display_name()
    );
    eprintln!("💡 Type 'exit' or press Ctrl+C to quit");

    let cli_command = get_cli_command(cli_type)?;

    // 构建交互模式参数
    let args = cli_type.build_interactive_args();
    let os_args: Vec<OsString> = args.into_iter().map(|s| s.into()).collect();

    let mut command = Command::new(&cli_command);
    command.args(&os_args);

    // 交互模式需要继承标准输入输出，让用户直接与CLI交互
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    platform::prepare_command(&mut command)?;

    // Inject environment variables to child process
    EnvInjector::inject_to_command(&mut command, &provider_config.env);

    // 直接启动进程并等待其完成
    let exit_status = command.status()?;
    Ok(exit_status.code().unwrap_or(1))
}
