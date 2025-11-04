use crate::cli_type::CliType;
use crate::logging::debug;
use crate::logging::warn;
use crate::platform::{self, ChildResources};
use crate::core::process_tree::{ProcessTreeError, ProcessTreeInfo};
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

/// Start interactive CLI mode (directly launch AI CLI without task prompt)
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

    let cli_command = get_cli_command(cli_type)?;

    // Interactive mode: launch CLI with stdin/stdout/stderr inherited
    let mut command = Command::new(&cli_command);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    platform::prepare_command(&mut command)?;

    // Inject environment variables to child process
    EnvInjector::inject_to_command(&mut command, &provider_config.env);

    let mut child = command.spawn()?;
    let child_pid = child.id();

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
        Ok(tree_info) => record.with_process_tree(
            tree_info.process_chain,
            tree_info.root_parent_pid,
            tree_info.depth,
        ),
        Err(err) => {
            warn(format!("Failed to get process tree info: {}", err));
            record
        }
    };

    if let Err(err) = registry.register(child_pid, &record) {
        platform::terminate_process(child_pid);
        let _ = child.wait();
        return Err(err.into());
    }

    let registration_guard = RegistrationGuard::new(registry, child_pid);
    let signal_guard = signal::install(child_pid)?;

    let status = child.wait()?;
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
pub fn execute_multiple_clis(
    registry: &TaskRegistry,
    cli_selector: &crate::cli_type::CliSelector,
    task_prompt: &str,
    provider: Option<String>,
) -> Result<Vec<i32>, ProcessError> {
    let mut exit_codes = Vec::new();

    for cli_type in &cli_selector.types {
        let cli_args = cli_type.build_full_access_args(task_prompt);
        let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

        let exit_code = execute_cli(registry, cli_type, &os_args, provider.clone())?;
        exit_codes.push(exit_code);
    }

    Ok(exit_codes)
}

