//! Grok CLI 安装探测
//!
//! 探测 ~/.grok/bin/grok 软链 → ~/.grok/downloads/grok-linux-x86_64，
//! 读 ~/.grok/version.json 得版本。

use crate::patcher::grok::versions::GrokVersion;
use crate::patcher::types::UnifiedPatchError;
use std::path::PathBuf;

/// Grok 安装信息
#[derive(Debug, Clone)]
pub struct GrokInstallation {
    pub binary_path: PathBuf,
    pub version: GrokVersion,
    pub installed: bool,
}

/// Grok home 目录（默认 ~/.grok）
fn grok_home() -> PathBuf {
    std::env::var("GROK_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".grok"))
}

/// Grok binary 路径: ~/.grok/downloads/grok-linux-x86_64
pub fn get_grok_binary_path() -> Result<PathBuf, UnifiedPatchError> {
    let binary = grok_home().join("downloads").join("grok-linux-x86_64");
    if binary.exists() {
        Ok(binary)
    } else {
        Err(UnifiedPatchError::FileError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Grok binary not found at ~/.grok/downloads/grok-linux-x86_64",
        )))
    }
}

/// 读 ~/.grok/version.json 得版本
fn read_grok_version() -> Option<GrokVersion> {
    let vjson = grok_home().join("version.json");
    let content = std::fs::read_to_string(&vjson).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let ver_str = v.get("version")?.as_str()?;
    GrokVersion::from_string(ver_str)
}

/// 探测 Grok 安装
pub fn detect_grok() -> Result<GrokInstallation, UnifiedPatchError> {
    let binary_path = get_grok_binary_path()?;
    // 优先读 version.json，失败则跑 grok --version
    let version = read_grok_version().unwrap_or_else(|| {
        let out = std::process::Command::new(&binary_path)
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        GrokVersion::from_string(&out).unwrap_or(GrokVersion { major: 0, minor: 0, patch: 0 })
    });
    Ok(GrokInstallation { binary_path, version, installed: true })
}
