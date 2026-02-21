use crate::cli_type::CliType;
use crate::core::models::ProcessTreeInfo;
use crate::core::process_tree::ProcessTreeError;
use crate::error::RegistryError;
use crate::logging::debug;
use crate::logging::warn;
#[cfg(windows)]
use crate::platform::ChildResources;
use crate::platform::{self};
use crate::provider::{AiType, ProviderManager};
use crate::signal;
use crate::storage::TaskStorage;
use crate::task_record::TaskRecord;
use crate::unified_registry::Registry;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
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

fn get_cli_command(cli_type: &CliType) -> Result<String, ProcessError> {
    let cmd_name = cli_type.command_name();

    match which::which(cmd_name) {
        Ok(path) => Ok(path.to_string_lossy().to_string()),
        Err(_) => Err(ProcessError::CliNotFound(format!(
            "'{}' not found in PATH",
            cmd_name
        ))),
    }
}

/// Output handling strategy for CLI execution
enum OutputStrategy {
    /// Mirror output to stdout/stderr (for interactive TTY)
    Mirror,
    /// Tail-only mode: capture all output, only emit last N lines at end (for non-TTY/piped output)
    /// This prevents flooding CC's context when stdout is captured by another program
    TailOnly,
    /// Capture stdout to buffer with display control
    CaptureWithDisplay(Arc<Mutex<Vec<u8>>>, Arc<Mutex<ScrollingDisplay>>),
    /// Legacy capture mode (for backward compatibility)
    Capture(Arc<Mutex<Vec<u8>>>),
    /// Capture stdout and stderr without mirroring output
    CaptureAll(Arc<Mutex<Vec<u8>>>, Arc<Mutex<Vec<u8>>>),
}

#[derive(Debug, Clone)]
pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

pub async fn execute_cli<S: TaskStorage>(
    registry: &Registry<S>,
    cli_type: &CliType,
    args: &[OsString],
    provider: Option<String>,
    cwd: Option<std::path::PathBuf>,
) -> Result<i32, ProcessError> {
    // 检测 stdout 是否是 TTY
    // 如果不是 TTY（被程序捕获，如 CC 的 Bash 工具），使用 TailOnly 模式
    // 这样可以防止大量输出冲爆 CC 的上下文
    let output_strategy = if std::io::stdout().is_terminal() {
        OutputStrategy::Mirror
    } else {
        OutputStrategy::TailOnly
    };

    execute_cli_internal(
        registry,
        cli_type,
        args,
        provider,
        None,
        output_strategy,
        cwd,
    )
    .await
    .map(|(exit_code, _)| exit_code)
}

/// Execute CLI and capture stdout output (for code generation)
/// 返回的内容限制为最后50行，符合规范要求
pub async fn execute_cli_with_output<S: TaskStorage>(
    registry: &Registry<S>,
    cli_type: &CliType,
    args: &[OsString],
    provider: Option<String>,
    timeout: std::time::Duration,
    cwd: Option<std::path::PathBuf>,
) -> Result<String, ProcessError> {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let scrolling_display = Arc::new(Mutex::new(ScrollingDisplay::new(DEFAULT_MAX_DISPLAY_LINES)));

    let (_, output_opt) = execute_cli_internal(
        registry,
        cli_type,
        args,
        provider,
        Some(timeout),
        OutputStrategy::CaptureWithDisplay(buffer.clone(), scrolling_display.clone()),
        cwd,
    )
    .await?;

    match output_opt {
        Some(output) => {
            let mut stdout = output.stdout;
            // 应用50行限制：只保留最后50行
            let lines: Vec<_> = stdout.lines().collect();
            if lines.len() > 50 {
                let limited_lines: Vec<_> =
                    lines.iter().rev().take(50).map(|s| s.to_string()).rev().collect();
                stdout = limited_lines.join("\n");
                if !stdout.ends_with('\n') {
                    stdout.push('\n');
                }
            }
            Ok(stdout)
        }
        None => Err(ProcessError::Other(
            "Output capture failed unexpectedly".to_string(),
        )),
    }
}

pub async fn execute_cli_with_full_output<S: TaskStorage>(
    registry: &Registry<S>,
    cli_type: &CliType,
    args: &[OsString],
    provider: Option<String>,
    timeout: std::time::Duration,
    cwd: Option<std::path::PathBuf>,
) -> Result<(i32, CapturedOutput), ProcessError> {
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Arc::new(Mutex::new(Vec::new()));

    let (exit_code, output_opt) = execute_cli_internal(
        registry,
        cli_type,
        args,
        provider,
        Some(timeout),
        OutputStrategy::CaptureAll(stdout.clone(), stderr.clone()),
        cwd,
    )
    .await?;

    let output = output_opt.ok_or_else(|| {
        ProcessError::Other("Output capture failed unexpectedly".to_string())
    })?;

    Ok((exit_code, output))
}

/// Internal CLI execution with configurable output handling
async fn execute_cli_internal<S: TaskStorage>(
    registry: &Registry<S>,
    cli_type: &CliType,
    args: &[OsString],
    provider: Option<String>,
    timeout: Option<std::time::Duration>,
    output_strategy: OutputStrategy,
    cwd: Option<std::path::PathBuf>,
) -> Result<(i32, Option<CapturedOutput>), ProcessError> {
    // Validate CWD if provided
    if let Some(ref dir) = cwd {
        if !dir.exists() {
            return Err(ProcessError::Other(format!(
                "Working directory does not exist: {}",
                dir.display()
            )));
        }
        if !dir.is_dir() {
            return Err(ProcessError::Other(format!(
                "Working directory is not a directory: {}",
                dir.display()
            )));
        }
    }
    let is_capture_mode = matches!(
        output_strategy,
        OutputStrategy::Capture(_)
            | OutputStrategy::CaptureWithDisplay(_, _)
            | OutputStrategy::CaptureAll(_, _)
    );

    platform::init_platform();

    let terminate_wrapper = |pid: u32| {
        platform::terminate_process(pid);
        Ok(())
    };
    registry.sweep_stale_entries(Utc::now(), platform::process_alive, &terminate_wrapper)?;

    // Load provider configuration
    let provider_manager = ProviderManager::new()
        .map_err(|e| ProcessError::Other(format!("Failed to load provider: {}", e)))?;

    // Determine AI type for compatibility checking
    let ai_type = match cli_type {
        CliType::Claude => AiType::Claude,
        CliType::Codex => AiType::Codex,
        CliType::Gemini => AiType::Gemini,
        CliType::Auto => {
            return Err(ProcessError::Other(
                "Auto CLI type is virtual and cannot be executed directly".to_string(),
            ))
        }
    };

    // Determine which provider to use
    let (provider_name, provider_config, is_fallback) = if let Some(ref name) = provider {
        if name.eq_ignore_ascii_case("auto") {
            // Auto mode: randomly select compatible provider
            if let Some((selected_name, config)) =
                provider_manager.get_random_compatible_provider(&ai_type)
            {
                eprintln!(
                    "Auto-selected provider: {} (for {})",
                    selected_name, ai_type
                );
                (selected_name, config, false)
            } else {
                // No compatible providers, fallback to no injection
                eprintln!(
                    "No compatible providers for {}, using native configuration",
                    ai_type
                );
                // Use official provider as placeholder (empty config, no env injection)
                let official = provider_manager
                    .get_provider("official")
                    .map_err(|e| ProcessError::Other(e.to_string()))?;
                ("".to_string(), official, true)
            }
        } else {
            // Normal mode: use specified provider
            let config = provider_manager
                .get_provider(name)
                .map_err(|e| ProcessError::Other(e.to_string()))?;
            (name.clone(), config, false)
        }
    } else {
        // Default provider
        let (name, config) = provider_manager
            .get_default_provider()
            .ok_or_else(|| ProcessError::Other("No default provider configured".to_string()))?;
        (name, config, false)
    };

    // Display provider info if not using official and not in fallback mode
    if !is_fallback && provider_name != *"official" && !provider_name.is_empty() {
        eprintln!(
            "Using provider: {} ({})",
            provider_name,
            provider_config.summary()
        );
    }

    let cli_command = get_cli_command(cli_type)?;

    let mut command = Command::new(&cli_command);
    command.args(args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // Remove nesting-detection env vars so child CLI processes don't think
    // they are running inside another session (e.g. Claude Code's CLAUDECODE check).
    command.env_remove("CLAUDECODE");
    command.env_remove("CLAUDE_CODE_ENTRYPOINT");

    // Set working directory if provided
    if let Some(ref dir) = cwd {
        command.current_dir(dir);
    }

    // Platform-specific command preparation
    #[cfg(unix)]
    {
        unsafe {
            command.pre_exec(|| {
                let result = libc::setpgid(0, 0);
                if result != 0 {
                    return Err(io::Error::last_os_error());
                }
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

    // Inject environment variables (skip in fallback mode)
    if !is_fallback {
        for (key, value) in &provider_config.env {
            command.env(key, value);
        }
        // For Codex with third-party providers, isolate CODEX_HOME to avoid
        // OAuth token conflicts from ~/.codex/auth.json
        if matches!(cli_type, CliType::Codex) && provider_name != "official" {
            setup_codex_home_for_provider(&mut command, &provider_config);
        }
    }

    let mut child = command.spawn()?;
    let child_pid = child
        .id()
        .ok_or_else(|| io::Error::other("Failed to get child PID"))?;

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
        "Started {} process pid={} log={}{}",
        cli_type.display_name(),
        child_pid,
        log_path.display(),
        if is_capture_mode {
            " (capture mode)"
        } else {
            ""
        }
    ));

    #[cfg(windows)]
    {
        let _resources = ChildResources::with_job(None);
    }

    let signal_guard = signal::install(child_pid)?;

    let log_writer = Arc::new(Mutex::new(BufWriter::new(log_file)));
    let mut copy_handles = Vec::new();

    // 创建共享的滚动显示缓冲区（stdout和stderr共享，保持输出顺序）
    let scrolling_display = Arc::new(Mutex::new(ScrollingDisplay::new(DEFAULT_MAX_DISPLAY_LINES)));

    // Handle stdout based on strategy
    if let Some(stdout) = child.stdout.take() {
        match &output_strategy {
            OutputStrategy::Mirror => {
                copy_handles.push(tokio::spawn(spawn_copy(
                    stdout,
                    log_writer.clone(),
                    StreamMirror::Stdout,
                    scrolling_display.clone(),
                )));
            }
            OutputStrategy::TailOnly => {
                // 静默收集输出，不实时写入终端，最后只输出最后 N 行
                let writer_clone = log_writer.clone();
                let display_clone = scrolling_display.clone();
                copy_handles.push(tokio::spawn(async move {
                    spawn_copy_silent(stdout, writer_clone, display_clone).await
                }));
            }
            OutputStrategy::CaptureWithDisplay(buffer, display) => {
                let buffer_clone = buffer.clone();
                let display_clone = display.clone();
                let writer_clone = log_writer.clone();
                copy_handles.push(tokio::spawn(async move {
                    spawn_copy_with_capture_and_display(stdout, writer_clone, buffer_clone, display_clone).await
                }));
            }
            OutputStrategy::Capture(buffer) => {
                let buffer_clone = buffer.clone();
                let writer_clone = log_writer.clone();
                copy_handles.push(tokio::spawn(async move {
                    spawn_copy_with_capture(stdout, writer_clone, buffer_clone).await
                }));
            }
            OutputStrategy::CaptureAll(buffer, _) => {
                let buffer_clone = buffer.clone();
                let writer_clone = log_writer.clone();
                copy_handles.push(tokio::spawn(async move {
                    spawn_copy_with_capture(stdout, writer_clone, buffer_clone).await
                }));
            }
        }
    }

    if let Some(stderr) = child.stderr.take() {
        match &output_strategy {
            OutputStrategy::TailOnly => {
                // TailOnly 模式：stderr 也静默收集
                let writer_clone = log_writer.clone();
                let display_clone = scrolling_display.clone();
                copy_handles.push(tokio::spawn(async move {
                    spawn_copy_silent(stderr, writer_clone, display_clone).await
                }));
            }
            OutputStrategy::CaptureAll(_, buffer) => {
                let buffer_clone = buffer.clone();
                let writer_clone = log_writer.clone();
                copy_handles.push(tokio::spawn(async move {
                    spawn_copy_with_capture(stderr, writer_clone, buffer_clone).await
                }));
            }
            _ => {
                // 其他模式：实时输出 stderr
                copy_handles.push(tokio::spawn(spawn_copy(
                    stderr,
                    log_writer.clone(),
                    StreamMirror::Stderr,
                    scrolling_display.clone(),
                )));
            }
        }
    }

    let registration_guard = {
        let mut record = TaskRecord::new(
            Utc::now(),
            child_pid.to_string(),
            log_path.to_string_lossy().into_owned(),
            Some(platform::current_pid()),
        );

        // Get process tree information
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
            }
        }

        if let Err(err) = registry.register(child_pid, &record) {
            platform::terminate_process(child_pid);
            let _ = child.wait().await;
            return Err(err.into());
        }
        Some(RegistrationGuard::new(registry, child_pid))
    };

    // Wait with optional timeout
    let status = if let Some(timeout_duration) = timeout {
        tokio::select! {
            result = child.wait() => result?,
            _ = tokio::time::sleep(timeout_duration) => {
                platform::terminate_process(child_pid);
                let _ = child.wait().await;
                return Err(ProcessError::Other(format!(
                    "CLI execution timed out after {:?}",
                    timeout_duration
                )));
            }
        }
    } else {
        child.wait().await?
    };

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

    // Only show completion info for non-interactive tasks (not capture mode)
    if !is_capture_mode {
        let mut display = scrolling_display.lock().await;
        let final_flush = display.flush_remaining();
        if !final_flush.is_empty() {
            let _ = tokio::io::stderr().write_all(final_flush.as_bytes()).await;
        }

        // 输出最后的50行
        match &output_strategy {
            OutputStrategy::TailOnly => {
                // 非 TTY 模式：输出纯文本（无 ANSI 转义码）
                let tail_output = display.get_plain_tail();
                if !tail_output.is_empty() {
                    let _ = tokio::io::stdout().write_all(tail_output.as_bytes()).await;
                }
            }
            OutputStrategy::Mirror => {
                // TTY 模式：使用 ANSI 转义码重绘
                let tail_output = display.redraw();
                if !tail_output.is_empty() {
                    let _ = tokio::io::stderr().write_all(tail_output.as_bytes()).await;
                }
            }
            _ => {
                // CaptureWithDisplay/Capture: 不在此处输出
            }
        }
    }

    if let Some(guard) = registration_guard {
        let completed_at = Utc::now();
        let exit_code = status.code();
        let result = match (status.success(), exit_code) {
            (true, _) => Some(if is_capture_mode {
                "codegen_success".to_owned()
            } else {
                "success".to_owned()
            }),
            (false, Some(code)) => Some(format!(
                "{}_failed_with_exit_code_{code}",
                if is_capture_mode { "codegen" } else { "cli" }
            )),
            (false, None) => Some(format!(
                "{}_failed_without_exit_code",
                if is_capture_mode { "codegen" } else { "cli" }
            )),
        };
        let _ = guard.mark_completed(result, exit_code, completed_at);
    }

    // Extract captured output if in capture mode
    let captured_output = match output_strategy {
        OutputStrategy::Capture(buffer) => {
            let output = buffer.lock().await.clone();
            let output_str = String::from_utf8_lossy(&output).to_string();

            if !status.success() {
                return Err(ProcessError::Other(format!(
                    "{} CLI failed with exit code {}: {}",
                    cli_type.display_name(),
                    extract_exit_code(status),
                    output_str
                )));
            }

            Some(CapturedOutput {
                stdout: output_str,
                stderr: String::new(),
            })
        }
        OutputStrategy::CaptureWithDisplay(buffer, _) => {
            let output = buffer.lock().await.clone();
            let output_str = String::from_utf8_lossy(&output).to_string();
            Some(CapturedOutput {
                stdout: output_str,
                stderr: String::new(),
            })
        }
        OutputStrategy::CaptureAll(stdout_buffer, stderr_buffer) => {
            let stdout_bytes = stdout_buffer.lock().await.clone();
            let stderr_bytes = stderr_buffer.lock().await.clone();
            Some(CapturedOutput {
                stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
                stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
            })
        }
        _ => None,
    };

    Ok((extract_exit_code(status), captured_output))
}

/// Generate a secure log file path in runtime directory
///
/// Security considerations:
/// - Uses system temp directory (cross-platform)
/// - Creates directory with restrictive permissions (0700 on Unix)
/// - Ensures logs are only accessible by the current user
/// Set up an isolated CODEX_HOME for third-party providers.
///
/// Codex CLI prioritizes OAuth tokens from `~/.codex/auth.json` over the
/// `OPENAI_API_KEY` environment variable. When using a third-party provider,
/// the OAuth token is invalid, causing 401 errors. This function creates a
/// temporary CODEX_HOME directory with a clean `auth.json` containing only
/// the provider's API key, and copies the user's `config.toml` for settings.
fn setup_codex_home_for_provider(
    command: &mut Command,
    provider_config: &crate::provider::config::Provider,
) {
    use std::fs;

    // Create a unique temp directory for this codex session
    let codex_home = std::env::temp_dir().join(format!(
        "aiw-codex-home-{}",
        std::process::id()
    ));
    if fs::create_dir_all(&codex_home).is_err() {
        return;
    }

    // Copy config.toml from original CODEX_HOME
    let original_home = dirs::home_dir()
        .map(|h| h.join(".codex"))
        .unwrap_or_default();
    let original_config = original_home.join("config.toml");
    if original_config.exists() {
        let _ = fs::copy(&original_config, codex_home.join("config.toml"));
    }

    // Write a clean auth.json with only the API key from provider env
    let api_key = provider_config
        .env
        .get("OPENAI_API_KEY")
        .cloned()
        .unwrap_or_default();
    let auth_json = if api_key.is_empty() {
        r#"{"OPENAI_API_KEY": null}"#.to_string()
    } else {
        format!(r#"{{"OPENAI_API_KEY": "{}"}}"#, api_key)
    };
    let _ = fs::write(codex_home.join("auth.json"), auth_json);

    command.env("CODEX_HOME", &codex_home);
}

/// - Logs are automatically cleaned up on system reboot
/// - Collision-resistant filename format: {PID}-{timestamp}-{random}.log
///   - PID: Process ID for process identification
///   - timestamp: Milliseconds since Unix epoch for time uniqueness
///   - random: Cryptographic random number for collision resistance
pub fn generate_log_path(pid: u32) -> io::Result<PathBuf> {
    // Use system temp directory as per SPEC design (cross-platform)
    // Linux/macOS: /tmp/.aiw/logs/
    // Windows: %TEMP%\.aiw\logs\
    // Runtime data (logs, temp files) → temp_dir()/.aiw/
    // Persistent config → ~/.aiw/
    let log_dir = std::env::temp_dir().join(".aiw").join("logs");

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

    // Generate collision-resistant filename with timestamp and random number
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let random = {
        let mut bytes = [0u8; 4];
        if getrandom::getrandom(&mut bytes).is_ok() {
            u32::from_be_bytes(bytes)
        } else {
            // Fallback to using PID + timestamp if random generation fails
            (pid ^ timestamp as u32).rotate_right(timestamp as u32 % 32)
        }
    };

    let filename = format!("{pid}-{}-{}.log", timestamp, random);
    Ok(log_dir.join(filename))
}

/// 滚动显示缓冲区 - 只在终端显示最后N行，完整内容保存到日志
pub struct ScrollingDisplay {
    lines: VecDeque<String>,
    max_lines: usize,
    pub current_line_buffer: String,
    pub displayed_count: usize,
}

impl ScrollingDisplay {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            current_line_buffer: String::new(),
            displayed_count: 0,
        }
    }

    /// 处理新数据，返回需要显示的内容
    pub fn process(&mut self, data: &[u8]) -> String {
        let text = String::from_utf8_lossy(data);
        let mut output = String::new();

        for ch in text.chars() {
            if ch == '\n' {
                // 完成一行
                let line = std::mem::take(&mut self.current_line_buffer);
                self.lines.push_back(line);

                // 严格限制在最大行数内，立即移除超过的行
                while self.lines.len() > self.max_lines {
                    self.lines.pop_front();
                }

                // 只有在刚达到最大行数时才需要重绘
                if self.lines.len() == self.max_lines {
                    output.push_str(&self.redraw());
                } else {
                    // 直接输出新行
                    if let Some(last) = self.lines.back() {
                        output.push_str(last);
                        output.push('\n');
                    }
                    self.displayed_count = self.lines.len();
                }
            } else if ch == '\r' {
                // 回车符，清除当前行缓冲
                self.current_line_buffer.clear();
            } else {
                self.current_line_buffer.push(ch);
            }
        }

        output
    }

    /// 重绘整个显示区域
    pub fn redraw(&mut self) -> String {
        let mut output = String::new();

        // 移动到显示区域顶部并清除
        if self.displayed_count > 0 {
            // 向上移动 displayed_count 行
            output.push_str(&format!("\x1b[{}A", self.displayed_count));
            // 清除从光标到屏幕底部
            output.push_str("\x1b[J");
        }

        // 输出所有行
        for line in &self.lines {
            output.push_str(line);
            output.push('\n');
        }

        self.displayed_count = self.lines.len();
        output
    }

    /// 刷新未完成的行（用于最终输出）
    pub fn flush_remaining(&mut self) -> String {
        if self.current_line_buffer.is_empty() {
            return String::new();
        }
        let line = std::mem::take(&mut self.current_line_buffer);
        format!("{}\n", line)
    }

    /// 获取纯文本尾部输出（无 ANSI 转义码，用于非 TTY 模式）
    /// 返回最后 N 行的纯文本，每行以换行符结尾
    pub fn get_plain_tail(&self) -> String {
        let mut output = String::new();
        for line in &self.lines {
            output.push_str(line);
            output.push('\n');
        }
        output
    }

    /// 验证当前显示是否严格符合50行限制
    pub fn validate_line_limit(&self) -> bool {
        self.lines.len() <= self.max_lines
    }

    /// 获取当前显示的行数
    pub fn current_line_count(&self) -> usize {
        self.lines.len()
    }
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

    async fn write_str(self, data: &str) -> io::Result<()> {
        self.write(data.as_bytes()).await
    }
}

/// 默认的最大显示行数
pub const DEFAULT_MAX_DISPLAY_LINES: usize = 50;

async fn spawn_copy<R>(
    mut reader: R,
    writer: Arc<Mutex<BufWriter<tokio::fs::File>>>,
    mirror: StreamMirror,
    scrolling_display: Arc<Mutex<ScrollingDisplay>>,
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

        // 写入完整日志文件
        {
            let mut guard = writer.lock().await;
            guard.write_all(chunk).await?;
            guard.flush().await?;
        }

        // 滚动显示到终端（只显示最后N行）
        let display_output = {
            let mut display = scrolling_display.lock().await;
            display.process(chunk)
        };
        if !display_output.is_empty() {
            mirror.write_str(&display_output).await?;
        }
    }

    // 刷新剩余未完成的行
    let remaining = {
        let mut display = scrolling_display.lock().await;
        display.flush_remaining()
    };
    if !remaining.is_empty() {
        mirror.write_str(&remaining).await?;
    }

    Ok(())
}

/// Copy stream to log file and scrolling display buffer, but do NOT mirror to stdout/stderr
/// Used for TailOnly mode: captures everything, outputs only last N lines at the end
async fn spawn_copy_silent<R>(
    mut reader: R,
    writer: Arc<Mutex<BufWriter<tokio::fs::File>>>,
    scrolling_display: Arc<Mutex<ScrollingDisplay>>,
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

        // 写入完整日志文件
        {
            let mut guard = writer.lock().await;
            guard.write_all(chunk).await?;
            guard.flush().await?;
        }

        // 收集到滚动缓冲区（但不输出到终端）
        {
            let mut display = scrolling_display.lock().await;
            // 调用 process 但丢弃输出（不写入终端）
            let _ = display.process(chunk);
        }
    }

    // 刷新剩余未完成的行到缓冲区
    {
        let mut display = scrolling_display.lock().await;
        let _ = display.flush_remaining();
    }

    Ok(())
}

/// Copy stream to log file and capture to buffer with display control (for code generation)
async fn spawn_copy_with_capture_and_display<R>(
    mut reader: R,
    writer: Arc<Mutex<BufWriter<tokio::fs::File>>>,
    capture_buffer: Arc<Mutex<Vec<u8>>>,
    display: Arc<Mutex<ScrollingDisplay>>,
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

        // Write to log file
        {
            let mut guard = writer.lock().await;
            guard.write_all(chunk).await?;
            guard.flush().await?;
        }

        // Capture to buffer
        {
            let mut capture = capture_buffer.lock().await;
            capture.extend_from_slice(chunk);
        }

        // Process through scrolling display (for line counting and limiting)
        {
            let mut display_guard = display.lock().await;
            display_guard.process(chunk);
        }
    }
    Ok(())
}

/// Copy stream to log file and capture to buffer (for code generation)
async fn spawn_copy_with_capture<R>(
    mut reader: R,
    writer: Arc<Mutex<BufWriter<tokio::fs::File>>>,
    capture_buffer: Arc<Mutex<Vec<u8>>>,
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

        // Write to log file
        {
            let mut guard = writer.lock().await;
            guard.write_all(chunk).await?;
            guard.flush().await?;
        }

        // Capture to buffer
        {
            let mut capture = capture_buffer.lock().await;
            capture.extend_from_slice(chunk);
        }
    }
    Ok(())
}

fn extract_exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

struct RegistrationGuard<'a, S: TaskStorage> {
    registry: &'a Registry<S>,
    pid: u32,
    active: bool,
}

impl<'a, S: TaskStorage> RegistrationGuard<'a, S> {
    fn new(registry: &'a Registry<S>, pid: u32) -> Self {
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

impl<S: TaskStorage> Drop for RegistrationGuard<'_, S> {
    fn drop(&mut self) {
        // 注意：TaskStorage trait不提供remove方法
        // 如果需要清理，应该通过mark_completed或sweep_stale_entries
        // 这里我们什么都不做，让任务记录保留在注册表中
    }
}

/// Start interactive CLI mode (directly launch AI CLI without task prompt)
pub async fn start_interactive_cli<S: TaskStorage>(
    registry: &Registry<S>,
    cli_type: &CliType,
    provider: Option<String>,
    cli_args: &[String],
    cwd: Option<std::path::PathBuf>,
) -> Result<i32, ProcessError> {
    // Validate CWD if provided
    if let Some(ref dir) = cwd {
        if !dir.exists() {
            return Err(ProcessError::Other(format!(
                "Working directory does not exist: {}",
                dir.display()
            )));
        }
        if !dir.is_dir() {
            return Err(ProcessError::Other(format!(
                "Working directory is not a directory: {}",
                dir.display()
            )));
        }
    }

    platform::init_platform();

    let terminate_wrapper = |pid: u32| {
        platform::terminate_process(pid);
        Ok(())
    };
    registry.sweep_stale_entries(Utc::now(), platform::process_alive, &terminate_wrapper)?;

    // Load provider configuration
    let provider_manager = ProviderManager::new()
        .map_err(|e| ProcessError::Other(format!("Failed to load provider: {}", e)))?;

    // Determine AI type for compatibility checking
    let ai_type = match cli_type {
        CliType::Claude => AiType::Claude,
        CliType::Codex => AiType::Codex,
        CliType::Gemini => AiType::Gemini,
        CliType::Auto => {
            return Err(ProcessError::Other(
                "Auto CLI type is virtual and cannot be executed directly".to_string(),
            ))
        }
    };

    // Determine which provider to use
    let (provider_name, provider_config, is_fallback) = if let Some(ref name) = provider {
        if name.eq_ignore_ascii_case("auto") {
            // Auto mode: randomly select compatible provider
            if let Some((selected_name, config)) =
                provider_manager.get_random_compatible_provider(&ai_type)
            {
                eprintln!(
                    "Auto-selected provider: {} (for {})",
                    selected_name, ai_type
                );
                (selected_name, config, false)
            } else {
                // No compatible providers, fallback to no injection
                eprintln!(
                    "No compatible providers for {}, using native configuration",
                    ai_type
                );
                // Use official provider as placeholder (empty config, no env injection)
                let official = provider_manager
                    .get_provider("official")
                    .map_err(|e| ProcessError::Other(e.to_string()))?;
                ("".to_string(), official, true)
            }
        } else {
            // Normal mode: use specified provider
            let config = provider_manager
                .get_provider(name)
                .map_err(|e| ProcessError::Other(e.to_string()))?;
            (name.clone(), config, false)
        }
    } else {
        // Default provider
        let (name, config) = provider_manager
            .get_default_provider()
            .ok_or_else(|| ProcessError::Other("No default provider configured".to_string()))?;
        (name, config, false)
    };

    // Display provider info if not using official and not in fallback mode
    if !is_fallback && provider_name != *"official" && !provider_name.is_empty() {
        eprintln!(
            "Using provider: {} ({})",
            provider_name,
            provider_config.summary()
        );
    }

    let cli_command = get_cli_command(cli_type)?;

    // Interactive mode: launch CLI with stdin/stdout/stderr inherited
    let mut command = Command::new(&cli_command);

    // Add interactive args (e.g., "exec" for Codex, "-p" for Claude)
    let interactive_args = cli_type.build_interactive_args_with_cli(cli_args);
    command.args(&interactive_args);

    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    // Remove nesting-detection env vars so child CLI processes don't think
    // they are running inside another session (e.g. Claude Code's CLAUDECODE check).
    command.env_remove("CLAUDECODE");
    command.env_remove("CLAUDE_CODE_ENTRYPOINT");

    // Set working directory if provided
    if let Some(ref dir) = cwd {
        command.current_dir(dir);
    }

    // Platform-specific command preparation (Unix: set process group and death signal)
    #[cfg(unix)]
    {
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

    // Inject environment variables (skip in fallback mode)
    if !is_fallback {
        for (key, value) in &provider_config.env {
            command.env(key, value);
        }
        // For Codex with third-party providers, isolate CODEX_HOME to avoid
        // OAuth token conflicts from ~/.codex/auth.json
        if matches!(cli_type, CliType::Codex) && provider_name != "official" {
            setup_codex_home_for_provider(&mut command, &provider_config);
        }
    }

    let mut child = command.spawn()?;
    let child_pid = child
        .id()
        .ok_or_else(|| io::Error::other("Failed to get child PID"))?;

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
pub async fn execute_multiple_clis<S: TaskStorage>(
    registry: &Registry<S>,
    cli_selector: &crate::cli_type::CliSelector,
    task_prompt: &str,
    provider: Option<String>,
    cli_args: &[String],
    cwd: Option<std::path::PathBuf>,
) -> Result<Vec<i32>, ProcessError> {
    let mut exit_codes = Vec::new();

    for cli_type in &cli_selector.types {
        let cli_args = cli_type.build_full_access_args_with_cli(task_prompt, cli_args);
        let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

        let exit_code = execute_cli(registry, cli_type, &os_args, provider.clone(), cwd.clone()).await?;
        exit_codes.push(exit_code);
    }

    Ok(exit_codes)
}
