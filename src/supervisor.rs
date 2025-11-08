use crate::cli_type::CliType;
use crate::core::models::ProcessTreeInfo;
use crate::core::process_tree::ProcessTreeError;
use crate::logging::debug;
use crate::logging::warn;
use crate::platform;
use crate::provider::{AiType, ProviderManager};
use crate::registry::{RegistryError, TaskRegistry};
use crate::signal;
use crate::task_record::TaskRecord;
use chrono::{DateTime, Utc};
use std::env;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use thiserror::Error;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncRead, AsyncWriteExt, BufWriter};
use tokio::process::Command;
use tokio::sync::Mutex;

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

async fn get_cli_command(cli_type: &CliType) -> Result<String, ProcessError> {
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
            .await
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
            .await
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

pub async fn execute_cli(
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

    let cli_command = get_cli_command(cli_type).await?;

    let mut command = Command::new(&cli_command);
    command.args(args);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // Platform-specific command preparation (Unix: set process group and death signal)
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                // Set process group ID
                let result = libc::setpgid(0, 0);
                if result != 0 {
                    return Err(io::Error::last_os_error());
                }
                // Set parent death signal on Linux
                #[cfg(target_os = "linux")]
                {
                    let result = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                    if result != 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
    }

    // Inject environment variables
    for (key, value) in &provider_config.env {
        command.env(key, value);
    }

    let mut child = command.spawn()?;
    let child_pid = child.id().ok_or_else(|| io::Error::other("Failed to get child PID"))?;

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
        .await
    {
        Ok(file) => file,
        Err(err) => {
            platform::terminate_process(child_pid);
            let _ = child.wait().await;
            return Err(err.into());
        }
    };

    debug(format!(
        "Started Codex process pid={} log={}",
        child_pid,
        log_path.display()
    ));

    // Note: ChildResources not needed with tokio::process::Child
    // It's only used on Windows for job object management
    #[cfg(windows)]
    {
        let _resources = ChildResources::new();
    }

    let signal_guard = signal::install(child_pid)?;

    let log_writer = Arc::new(Mutex::new(BufWriter::new(log_file)));
    let mut copy_handles = Vec::new();

    if let Some(stdout) = child.stdout.take() {
        copy_handles.push(tokio::spawn(spawn_copy(stdout, log_writer.clone(), StreamMirror::Stdout)));
    }
    if let Some(stderr) = child.stderr.take() {
        copy_handles.push(tokio::spawn(spawn_copy(stderr, log_writer.clone(), StreamMirror::Stderr)));
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
            Ok(tree_info) => match record.clone().with_process_tree_info(tree_info) {
                Ok(updated) => {
                    record = updated;
                }
                Err(err) => {
                    warn(format!("Failed to attach process tree info: {}", err));
                }
            },
            Err(err) => {
                warn(format!("Failed to get process tree info: {}", err));
                // Continue with basic record creation
            }
        }

        if let Err(err) = registry.register(child_pid, &record) {
            platform::terminate_process(child_pid);
            let _ = child.wait().await;
            return Err(err.into());
        }
        Some(RegistrationGuard::new(registry, child_pid))
    } else {
        None
    };

    let status = child.wait().await?;
    drop(signal_guard);

    for handle in copy_handles {
        match handle.await {
            Ok(result) => result?,
            Err(_) => {
                return Err(io::Error::other("Log writer task failed").into());
            }
        }
    }

    {
        let mut writer = log_writer.lock().await;
        writer.flush().await?;
        writer.get_ref().sync_all().await?;
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

/// Generate a secure log file path in a user-specific directory
///
/// Security considerations:
/// - Uses user-specific directory (~/.agentic-warden/logs/) instead of shared /tmp
/// - Creates directory with restrictive permissions (0700 on Unix)
/// - Ensures logs are only accessible by the current user
fn generate_log_path(pid: u32) -> io::Result<PathBuf> {
    // Use user-specific directory instead of shared temp directory
    let log_dir = if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("agentic-warden").join("logs")
    } else {
        // Fallback to home directory if config_dir is not available
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".agentic-warden").join("logs")
    };

    // Create the logs directory if it doesn't exist
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir)?;

        // Set restrictive permissions on Unix systems (only user can read/write/execute)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&log_dir)?.permissions();
            perms.set_mode(0o700); // rwx------
            std::fs::set_permissions(&log_dir, perms)?;
        }
    }

    Ok(log_dir.join(format!("{pid}.log")))
}

#[derive(Copy, Clone)]
enum StreamMirror {
    Stdout,
    Stderr,
}

impl StreamMirror {
    async fn write(self, data: &[u8]) -> io::Result<()> {
        use tokio::io::AsyncWriteExt;
        match self {
            StreamMirror::Stdout => {
                let mut handle = tokio::io::stdout();
                handle.write_all(data).await?;
                handle.flush().await
            }
            StreamMirror::Stderr => {
                let mut handle = tokio::io::stderr();
                handle.write_all(data).await?;
                handle.flush().await
            }
        }
    }
}

async fn spawn_copy<R>(
    mut reader: R,
    writer: Arc<Mutex<BufWriter<tokio::fs::File>>>,
    mirror: StreamMirror,
) -> io::Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    use tokio::io::AsyncReadExt;

    let mut buffer = [0u8; 8192];
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        {
            let mut guard = writer.lock().await;
            guard.write_all(chunk).await?;
            guard.flush().await?;
        }
        mirror.write(chunk).await?;
    }
    Ok(())
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

/// Start interactive CLI mode (directly launch AI CLI without task prompt)
pub async fn start_interactive_cli(
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

    let cli_command = get_cli_command(cli_type).await?;

    // Interactive mode: launch CLI with stdin/stdout/stderr inherited
    let mut command = Command::new(&cli_command);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    // Platform-specific command preparation (Unix: set process group and death signal)
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                // Set process group ID
                let result = libc::setpgid(0, 0);
                if result != 0 {
                    return Err(io::Error::last_os_error());
                }
                // Set parent death signal on Linux
                #[cfg(target_os = "linux")]
                {
                    let result = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                    if result != 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
    }

    // Inject environment variables
    for (key, value) in &provider_config.env {
        command.env(key, value);
    }

    let mut child = command.spawn()?;
    let child_pid = child.id().ok_or_else(|| io::Error::other("Failed to get child PID"))?;

    // Register the interactive CLI process
    let log_path = generate_log_path(child_pid)?;
    let record = TaskRecord::new(
        Utc::now(),
        child_pid.to_string(),
        log_path.to_string_lossy().into_owned(),
        Some(platform::current_pid()),
    );

    // Get process tree information
    let record = match ProcessTreeInfo::current() {
        Ok(tree_info) => match record.clone().with_process_tree_info(tree_info) {
            Ok(updated) => updated,
            Err(err) => {
                warn(format!("Failed to attach process tree info: {}", err));
                record
            }
        },
        Err(err) => {
            warn(format!("Failed to get process tree info: {}", err));
            record
        }
    };

    if let Err(err) = registry.register(child_pid, &record) {
        platform::terminate_process(child_pid);
        let _ = child.wait().await;
        return Err(err.into());
    }

    let registration_guard = RegistrationGuard::new(registry, child_pid);
    let signal_guard = signal::install(child_pid)?;

    let status = child.wait().await?;
    drop(signal_guard);

    // Mark as completed
    let completed_at = Utc::now();
    let exit_code = status.code();
    let result = match (status.success(), exit_code) {
        (true, _) => Some("interactive_session_completed".to_owned()),
        (false, Some(code)) => Some(format!("interactive_session_failed_with_exit_code_{code}")),
        (false, None) => Some("interactive_session_failed_without_exit_code".to_owned()),
    };
    let _ = registration_guard.mark_completed(result, exit_code, completed_at);

    Ok(extract_exit_code(status))
}

/// Execute multiple CLI processes (for codex|claude|gemini syntax)
pub async fn execute_multiple_clis(
    registry: &TaskRegistry,
    cli_selector: &crate::cli_type::CliSelector,
    task_prompt: &str,
    provider: Option<String>,
) -> Result<Vec<i32>, ProcessError> {
    let mut exit_codes = Vec::new();

    for cli_type in &cli_selector.types {
        let cli_args = cli_type.build_full_access_args(task_prompt);
        let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

        let exit_code = execute_cli(registry, cli_type, &os_args, provider.clone()).await?;
        exit_codes.push(exit_code);
    }

    Ok(exit_codes)
}
