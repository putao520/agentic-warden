use agentic_warden::cli_type::CliType;
use agentic_warden::commands::ai_cli::AiCliCommand;
use agentic_warden::registry_factory::RegistryFactory;
use agentic_warden::supervisor;
use agentic_warden::ProcessError;
use serial_test::serial;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tempfile::TempDir;
use which::which;

/// Scenario: Overriding `CODEX_BIN` with a non-existent path should trigger the same IO error a real user would hit.
///
/// Action: Point `CODEX_BIN` to an invalid drive location and invoke `supervisor::execute_cli` with real arguments.
///
/// Expectation: The launcher surfaces `ProcessError::Io` with `io::ErrorKind::NotFound`, proving a real spawn was attempted.
#[tokio::test]
#[serial]
async fn codex_env_override_to_missing_binary_returns_os_error() {
    ensure_cli_available("codex");
    let _home = TempHome::new();
    let _guard = EnvGuard::set(
        "CODEX_BIN",
        Path::new("Z:/definitely/missing/codex.exe").as_os_str(),
    );

    let registry = RegistryFactory::instance().get_cli_registry().expect("task registry should connect");
    let args = prompt_args(&CliType::Codex, "test");

    let err = supervisor::execute_cli(&registry, &CliType::Codex, &args, None).await
        .expect_err("missing binary must surface as IO error");

    match err {
        ProcessError::Io(io_err) => assert_eq!(io_err.kind(), io::ErrorKind::NotFound),
        other => panic!("expected IO error, got {other:?}"),
    }
}

/// Scenario: An empty `CLAUDE_BIN` should be treated as a hard error even when the binary exists on PATH.
///
/// Action: Set `CLAUDE_BIN` to an empty string and call `supervisor::execute_cli`.
///
/// Expectation: The supervisor returns `ProcessError::CliNotFound` with a clear message.
#[tokio::test]
#[serial]
async fn claude_empty_env_path_is_rejected_before_spawn() {
    ensure_cli_available("claude");
    let _home = TempHome::new();
    let _guard = EnvGuard::set("CLAUDE_BIN", OsStr::new(""));

    let registry = RegistryFactory::instance().get_cli_registry().expect("task registry should connect");
    let args = prompt_args(&CliType::Claude, "test");

    let err = supervisor::execute_cli(&registry, &CliType::Claude, &args, None).await
        .expect_err("empty path must be rejected");

    match err {
        ProcessError::CliNotFound(message) => {
            assert!(
                message.contains("CLAUDE_BIN environment variable is empty"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected CliNotFound error, got {other:?}"),
    }
}

/// Scenario: When PATH no longer contains Gemini, the supervisor should rely on the real `where/which` result.
///
/// Action: Remove `GEMINI_BIN`, shrink PATH to the bare system directory, and attempt to execute Gemini.
///
/// Expectation: `ProcessError::CliNotFound` reports that `gemini` is not present in PATH.
#[tokio::test]
#[serial]
async fn gemini_removed_from_path_reports_not_found() {
    ensure_cli_available("gemini");
    let _home = TempHome::new();
    let _bin_guard = EnvGuard::unset("GEMINI_BIN");
    let system_path = minimal_system_path();
    let _path_guard = EnvGuard::set("PATH", system_path.as_os_str());

    let registry = RegistryFactory::instance().get_cli_registry().expect("task registry should connect");
    let args = prompt_args(&CliType::Gemini, "test");

    let err = supervisor::execute_cli(&registry, &CliType::Gemini, &args, None).await
        .expect_err("missing binary on PATH must be reported");

    match err {
        ProcessError::CliNotFound(message) => {
            assert!(
                message.contains("'gemini' not found in PATH")
                    || message.contains("Failed to check if 'gemini' exists in PATH"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected CliNotFound error, got {other:?}"),
    }
}

/// Scenario: Passing a provider ID that does not exist in providers.json must bubble a descriptive error.
///
/// Action: Launch Codex with `provider=ghost` using the real supervisor logic.
///
/// Expectation: The call fails with `ProcessError::Other` mentioning the missing provider.
#[tokio::test]
#[serial]
async fn execute_cli_with_unknown_provider_surfaces_error() {
    ensure_cli_available("codex");
    let _home = TempHome::new();

    let registry = RegistryFactory::instance().get_cli_registry().expect("task registry should connect");
    let args = prompt_args(&CliType::Codex, "test");

    let err = supervisor::execute_cli(&registry, &CliType::Codex, &args, Some("ghost".to_string())).await
        .expect_err("unknown provider must fail");

    match err {
        ProcessError::Other(message) => {
            assert!(
                message.contains("Provider 'ghost' not found in configuration"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected provider error, got {other:?}"),
    }
}

/// Scenario: The temporary directory is invalid so the supervisor cannot create its log file.
///
/// Action: Point `TMP`/`TEMP` to a non-existent drive and attempt to launch Codex.
///
/// Expectation: `supervisor::execute_cli` returns `ProcessError::Io` due to log path creation failure.
#[tokio::test]
#[serial]
async fn execute_cli_returns_error_when_tmp_dir_is_unusable() {
    ensure_cli_available("codex");
    let _home = TempHome::new();
    let registry = RegistryFactory::instance().get_cli_registry().expect("task registry should connect");
    let fake_root = TempDir::new().expect("temp sandbox for tmp override");
    let file_path = fake_root.path().join("tmp-anchor");
    fs::write(&file_path, b"x").expect("create fake temp file");
    let _tmp_guard = EnvGuard::set("TMP", file_path.as_os_str());
    let _temp_guard = EnvGuard::set("TEMP", file_path.as_os_str());
    let args = prompt_args(&CliType::Codex, "test");

    let err = supervisor::execute_cli(&registry, &CliType::Codex, &args, None).await
        .expect_err("invalid temp dir should cause IO error");

    match err {
        ProcessError::Io(io_err) => {
            assert!(
                io_err.kind() == io::ErrorKind::NotFound
                    || io_err.kind() == io::ErrorKind::PermissionDenied,
                "unexpected IO error: {io_err}"
            );
        }
        other => panic!("expected IO error, got {other:?}"),
    }
}

/// Scenario: Multi-CLI execution should return the first non-zero exit code when the first CLI fails fast.
///
/// Action: Use an invalid flag to make Codex fail immediately, then test the batching logic.
///
/// Expectation: The combined command exits with the same non-zero code as the first CLI failure.
#[tokio::test]
#[serial]
async fn multi_cli_mode_returns_first_non_zero_exit_code() {
    ensure_cli_available("codex");
    let _home = TempHome::new();

    // Test with invalid flag that causes immediate failure
    let registry = RegistryFactory::instance().get_cli_registry().expect("task registry should connect");
    let invalid_args = prompt_args(&CliType::Codex, "--definitely-invalid-flag");
    let codex_exit = supervisor::execute_cli(&registry, &CliType::Codex, &invalid_args, None).await
        .expect("codex should execute and return exit status");
    assert_ne!(
        codex_exit, 0,
        "invalid flag should produce non-zero exit code"
    );

    // Test that AiCliCommand handles the same failure
    let command = AiCliCommand::new(
        vec![CliType::Codex, CliType::Codex],
        None,
        "--definitely-invalid-flag".to_string(),
    );
    let exit_code = command.execute().await
        .expect("ai cli execution should finish");

    assert_eq!(exit_code, ExitCode::from((codex_exit & 0xFF) as u8));
}

/// Scenario: Interactive mode must reject multiple CLI selections when the prompt is empty.
///
/// Action: Build an `AiCliCommand` with two CLI types and an empty prompt, then execute it.
///
/// Expectation: The call returns an error explaining that interactive mode only supports a single CLI.
#[tokio::test]
#[serial]
async fn interactive_mode_rejects_multiple_cli_without_prompt() {
    let command = AiCliCommand::new(vec![CliType::Codex, CliType::Gemini], None, String::new());
    let err = command.execute().await
        .expect_err("interactive multi CLI should be rejected");

    assert!(
        err.to_string()
            .contains("Interactive mode only supports single CLI"),
        "unexpected error message: {err}"
    );
}

/// Scenario: Running the real Gemini CLI with an invalid flag should propagate the non-zero exit code unchanged.
///
/// Action: Execute Gemini through the supervisor with a bogus flag that the CLI rejects.
///
/// Expectation: `supervisor::execute_cli` returns `Ok(exit_code)` and the exit code is non-zero.
#[tokio::test]
#[serial]
async fn execute_cli_propagates_real_gemini_exit_status() {
    ensure_cli_available("gemini");
    let _home = TempHome::new();

    let registry = RegistryFactory::instance().get_cli_registry().expect("task registry should connect");
    let args = prompt_args(&CliType::Gemini, "--definitely-invalid-flag");

    let exit_code = supervisor::execute_cli(&registry, &CliType::Gemini, &args, None).await
        .expect("gemini command should execute and return exit status");

    assert_ne!(
        exit_code, 0,
        "invalid flag should produce non-zero exit code"
    );
}

fn ensure_cli_available(binary: &str) {
    which(binary).unwrap_or_else(|_| panic!("expected {binary} to be installed on PATH"));
}

fn prompt_args(cli_type: &CliType, prompt: &str) -> Vec<OsString> {
    cli_type
        .build_full_access_args(prompt)
        .into_iter()
        .map(OsString::from)
        .collect()
}

fn minimal_system_path() -> PathBuf {
    if cfg!(windows) {
        let system_root = env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:\\Windows"));
        system_root.join("System32")
    } else {
        PathBuf::from("/usr/bin")
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
        guards.push(EnvGuard::set("HOME", dir.path().as_os_str()));
        guards.push(EnvGuard::set("USERPROFILE", dir.path().as_os_str()));
        #[cfg(windows)]
        {
            if let Some((drive, path)) = split_windows_home(dir.path()) {
                guards.push(EnvGuard::set("HOMEDRIVE", &drive));
                guards.push(EnvGuard::set("HOMEPATH", &path));
            }
        }
        Self {
            _dir: dir,
            _guards: guards,
        }
    }
}

#[cfg(windows)]
fn split_windows_home(path: &Path) -> Option<(OsString, OsString)> {
    let s = path.to_str()?;
    if s.len() < 2 || !s.as_bytes()[1].eq(&b':') {
        return None;
    }
    let drive = OsString::from(&s[..2]);
    let mut rest = s[2..].replace('/', "\\");
    if !rest.starts_with('\\') {
        rest = format!("\\{rest}");
    }
    Some((drive, OsString::from(rest)))
}

struct EnvGuard {
    key: String,
    original: Option<OsString>,
}

impl EnvGuard {
    fn set(key: impl Into<String>, value: impl AsRef<OsStr>) -> Self {
        let key_string = key.into();
        let original = env::var_os(&key_string);
        let new_value = OsString::from(value.as_ref());
        env::set_var(&key_string, &new_value);
        Self {
            key: key_string,
            original,
        }
    }

    fn unset(key: impl Into<String>) -> Self {
        let key_string = key.into();
        let original = env::var_os(&key_string);
        env::remove_var(&key_string);
        Self {
            key: key_string,
            original,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => env::set_var(&self.key, value),
            None => env::remove_var(&self.key),
        }
    }
}
