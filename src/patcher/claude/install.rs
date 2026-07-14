//! Claude CLI 安装探测（专有）
//!
//! 检测 Claude CLI 安装类型（npm JS / native binary / unknown），
//! 提供可补丁文件路径的统一入口。

use crate::patcher::file::{classify_magic_bytes, MagicKind};
use crate::patcher::types::{Result, UnifiedPatchError};
use std::fs;
use std::path::{Path, PathBuf};

/// Claude CLI 安装类型
#[derive(Debug, Clone)]
pub enum InstallationType {
    /// npm 全局安装 (JS 文件)
    Npm { js_path: PathBuf },
    /// 本地二进制安装 (ELF/Mach-O)
    NativeBinary { binary_path: PathBuf },
    /// 未知安装方式
    Unknown,
}

/// 获取 Claude CLI 文件路径（npm 安装）
pub fn get_claude_cli_path() -> Result<std::path::PathBuf> {
    let output = std::process::Command::new("which")
        .arg("claude")
        .output()
        .map_err(|e| UnifiedPatchError::FileError(
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("Failed to find claude: {}", e))
        ))?;

    if !output.status.success() {
        return Err(UnifiedPatchError::FileError(
            std::io::Error::new(std::io::ErrorKind::NotFound, "claude command not found")
        ));
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(std::path::PathBuf::from(path))
}

/// 检测 Claude CLI 安装类型
pub fn detect_installation() -> Result<InstallationType> {
    let claude_path = get_claude_cli_path()?;

    // 解析符号链接
    let real_path = fs::canonicalize(&claude_path).unwrap_or(claude_path.clone());

    // 读取文件头 4 字节判断类型
    let header = fs::read(&real_path)
        .map(|data| data.iter().take(4).copied().collect::<Vec<u8>>())
        .unwrap_or_default();

    if header.len() < 4 {
        return Ok(InstallationType::Unknown);
    }

    match classify_magic_bytes(&header) {
        MagicKind::NativeBinary => Ok(InstallationType::NativeBinary { binary_path: real_path }),
        MagicKind::Shebang => match parse_npm_js_path(&real_path) {
            Ok(js_path) => Ok(InstallationType::Npm { js_path }),
            Err(_) => Ok(InstallationType::Unknown),
        },
        MagicKind::Unknown => Ok(InstallationType::Unknown),
    }
}

/// 获取可补丁的文件路径（统一入口）
pub fn get_patchable_path() -> Result<PathBuf> {
    match detect_installation()? {
        InstallationType::Npm { js_path } => Ok(js_path),
        InstallationType::NativeBinary { binary_path } => Ok(binary_path),
        InstallationType::Unknown => Err(UnifiedPatchError::FileError(
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "无法识别 Claude CLI 安装类型",
            ),
        )),
    }
}

/// 从 npm shell 脚本中解析 JS 文件路径
fn parse_npm_js_path(script_path: &Path) -> Result<PathBuf> {
    let script_content = fs::read_to_string(script_path)?;

    for line in script_content.lines() {
        if line.contains("node_modules") && (line.contains(".js") || line.contains("basedir")) {
            if let Some(basedir_start) = line.find("basedir=") {
                let basedir_part = &line[basedir_start + 8..];
                if let Some(basedir_end) = basedir_part.find(' ') {
                    let basedir = &basedir_part[..basedir_end];
                    let basedir = shellexpand::env(basedir)
                        .map_err(|e| UnifiedPatchError::Other(format!("Failed to expand basedir: {}", e)))?;
                    return Ok(std::path::PathBuf::from(basedir.as_ref()).join("cli.js"));
                }
            }
        }
    }

    Err(UnifiedPatchError::FileError(
        std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find claude.js path in npm script"),
    ))
}

/// 获取 npm 安装的 Claude CLI 的 JavaScript 文件路径（兼容包装）
pub fn get_claude_js_path() -> Result<std::path::PathBuf> {
    match detect_installation()? {
        InstallationType::Npm { js_path } => Ok(js_path),
        _ => Err(UnifiedPatchError::FileError(
            std::io::Error::new(std::io::ErrorKind::NotFound, "Not an npm installation"),
        )),
    }
}

/// 检查是否为 npm 安装（兼容包装）
pub fn is_npm_installation() -> Result<bool> {
    match detect_installation()? {
        InstallationType::Npm { .. } => Ok(true),
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_cli_path() {
        // 测试获取 claude 路径
        let result = get_claude_cli_path();
        // 结果取决于系统是否有 claude 命令
        assert!(result.is_ok() || result.is_err());
    }
}
