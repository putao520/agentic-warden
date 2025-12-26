use aiw::mcp_routing::config::{McpConfig, McpServerConfig};
use aiw::tui::screens::{InstalledMcpScreen, Screen};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fake::faker::lorem::en::Word;
use fake::Fake;
use ratatui::{backend::TestBackend, Terminal};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct TempHome {
    dir: TempDir,
    old_home: Option<std::ffi::OsString>,
    old_user: Option<std::ffi::OsString>,
    old_openai_token: Option<std::ffi::OsString>,
    old_openai_endpoint: Option<std::ffi::OsString>,
    old_openai_model: Option<std::ffi::OsString>,
}

impl TempHome {
    pub fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().to_path_buf();
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &path);
        let old_user = std::env::var_os("USERPROFILE");
        if cfg!(windows) {
            std::env::set_var("USERPROFILE", &path);
        }
        let old_openai_token = std::env::var_os("OPENAI_TOKEN");
        let old_openai_endpoint = std::env::var_os("OPENAI_ENDPOINT");
        let old_openai_model = std::env::var_os("OPENAI_MODEL");
        std::env::remove_var("OPENAI_TOKEN");
        std::env::remove_var("OPENAI_ENDPOINT");
        std::env::remove_var("OPENAI_MODEL");
        Self {
            dir,
            old_home,
            old_user,
            old_openai_token,
            old_openai_endpoint,
            old_openai_model,
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        if let Some(old) = self.old_home.take() {
            std::env::set_var("HOME", old);
        } else {
            std::env::remove_var("HOME");
        }
        if cfg!(windows) {
            if let Some(old) = self.old_user.take() {
                std::env::set_var("USERPROFILE", old);
            } else {
                std::env::remove_var("USERPROFILE");
            }
        }
        if let Some(old) = self.old_openai_token.take() {
            std::env::set_var("OPENAI_TOKEN", old);
        } else {
            std::env::remove_var("OPENAI_TOKEN");
        }
        if let Some(old) = self.old_openai_endpoint.take() {
            std::env::set_var("OPENAI_ENDPOINT", old);
        } else {
            std::env::remove_var("OPENAI_ENDPOINT");
        }
        if let Some(old) = self.old_openai_model.take() {
            std::env::set_var("OPENAI_MODEL", old);
        } else {
            std::env::remove_var("OPENAI_MODEL");
        }
    }
}

pub struct ServerFixture {
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub source: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

pub fn write_mcp_config(home: &TempHome, servers: Vec<ServerFixture>) -> Result<PathBuf> {
    let aiw_dir = home.path().join(".aiw");
    fs::create_dir_all(&aiw_dir)?;
    let mut mcp_servers = HashMap::new();
    for server in servers {
        mcp_servers.insert(
            server.name,
            McpServerConfig {
                command: server.command,
                args: server.args,
                env: server.env,
                description: server.description,
                category: None,
                enabled: server.enabled,
                health_check: None,
                source: server.source,
            },
        );
    }
    let config = McpConfig {
        version: "1.0".to_string(),
        mcp_servers,
    };
    let config_path = aiw_dir.join("mcp.json");
    fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(config_path)
}

pub fn render_installed_screen(screen: &mut InstalledMcpScreen) -> String {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| screen.render(frame, frame.size()))
        .expect("render screen");

    let mut rendered = String::new();
    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered.push_str(buffer.get(x, y).symbol());
        }
        rendered.push('\n');
    }
    rendered
}

pub fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub fn send_text(screen: &mut InstalledMcpScreen, text: &str) -> Result<()> {
    for ch in text.chars() {
        screen.handle_key(key(KeyCode::Char(ch)))?;
    }
    Ok(())
}

pub fn send_backspaces(screen: &mut InstalledMcpScreen, count: usize) -> Result<()> {
    for _ in 0..count {
        screen.handle_key(key(KeyCode::Backspace))?;
    }
    Ok(())
}

pub fn set_read_only(path: &Path, readonly: bool) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(readonly);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

pub fn fake_word() -> String {
    let word: String = Word().fake();
    word.to_ascii_lowercase()
}

pub fn fake_server_name() -> String {
    format!("mcp-{}", fake_word())
}

pub fn fake_description() -> String {
    format!("{} {}", fake_word(), fake_word())
}

pub fn fake_env_key() -> String {
    format!("AIW_{}", fake_word().to_ascii_uppercase())
}

pub fn fake_env_value() -> String {
    format!("val_{}", fake_word())
}

pub fn fake_command() -> String {
    format!("cmd-{}", fake_word())
}
