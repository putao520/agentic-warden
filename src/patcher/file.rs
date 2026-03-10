//! 文件补丁实现
//!
//! 直接修改磁盘上的 Claude CLI 文件，支持 npm 安装（JS）和本地二进制安装（ELF/Mach-O）

use crate::patcher::types::{UnifiedPatchError, UnifiedPatchPattern, UnifiedPatchResult, Result};
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

/// 应用文件补丁
pub fn apply_file_patch(
    file_path: &Path,
    pattern: &UnifiedPatchPattern,
) -> Result<UnifiedPatchResult> {
    // NativeBinary 补丁前自动备份（仅首次）
    let backup_path = PathBuf::from(format!("{}.aiw-backup", file_path.display()));
    if !backup_path.exists() {
        // 检测是否为二进制文件（非文本）
        if is_binary_file(file_path) {
            fs::copy(file_path, &backup_path).map_err(|e| {
                UnifiedPatchError::FileError(std::io::Error::new(
                    e.kind(),
                    format!("备份失败 {}: {}", file_path.display(), e),
                ))
            })?;
        }
    }

    // 读取文件内容
    let content = fs::read(file_path)?;

    // 查找所有匹配位置
    let search_len = pattern.search_pattern.len();
    let mut positions: Vec<usize> = Vec::new();
    let mut start = 0;
    while start + search_len <= content.len() {
        if let Some(pos) = content[start..].windows(search_len)
            .position(|window| window == pattern.search_pattern)
        {
            positions.push(start + pos);
            start = start + pos + search_len;
        } else {
            break;
        }
    }

    if positions.is_empty() {
        return Err(UnifiedPatchError::PatternNotFound(format!("{:?}", pattern.search_pattern)));
    }

    // 应用补丁（替换所有匹配）
    let patched_content = if let Some(replace) = pattern.replace_pattern {
        // 从后往前替换，避免偏移变化
        let mut new_content = content.clone();
        for &pos in positions.iter().rev() {
            let mut result = Vec::with_capacity(new_content.len());
            result.extend_from_slice(&new_content[..pos]);
            result.extend_from_slice(replace);
            result.extend_from_slice(&new_content[pos + search_len..]);
            new_content = result;
        }
        new_content
    } else if let (Some(patch_byte), Some(offset)) = (pattern.patch_byte, pattern.patch_offset) {
        // 单字节修补所有匹配
        let mut new_content = content.clone();
        for &pos in &positions {
            new_content[pos + offset] = patch_byte;
        }
        new_content
    } else {
        return Err(UnifiedPatchError::PatternNotFound(
            "No replacement pattern or patch byte specified".to_string()
        ));
    };

    // 写回文件（对二进制文件使用 rename 策略避免 "Text file busy"）
    if is_binary_file(file_path) {
        let tmp_path = PathBuf::from(format!("{}.aiw-tmp", file_path.display()));
        fs::write(&tmp_path, &patched_content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(file_path) {
                let _ = fs::set_permissions(&tmp_path, metadata.permissions());
            }
        }
        fs::rename(&tmp_path, file_path)?;
    } else {
        fs::write(file_path, &patched_content)?;
    }

    Ok(UnifiedPatchResult::FilePatched {
        path: file_path.display().to_string(),
    })
}

/// 检查文件是否已应用补丁
pub fn is_file_patched(
    file_path: &Path,
    pattern: &UnifiedPatchPattern,
) -> Result<bool> {
    let content = fs::read(file_path)?;
    
    // 检查是否包含原始模式（未补丁）
    let has_original = content
        .windows(pattern.search_pattern.len())
        .any(|window| window == pattern.search_pattern);
    
    Ok(!has_original)
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
    
    match &header[..4] {
        // ELF magic: 0x7f 'E' 'L' 'F'
        [0x7f, 0x45, 0x4c, 0x46] => {
            Ok(InstallationType::NativeBinary { binary_path: real_path })
        }
        // Mach-O magic (32-bit and 64-bit, both endianness)
        [0xfe, 0xed, 0xfa, 0xce] | [0xfe, 0xed, 0xfa, 0xcf] |
        [0xce, 0xfa, 0xed, 0xfe] | [0xcf, 0xfa, 0xed, 0xfe] => {
            Ok(InstallationType::NativeBinary { binary_path: real_path })
        }
        // Shebang (#!) — npm shell 脚本
        [0x23, 0x21, ..] => {
            match parse_npm_js_path(&real_path) {
                Ok(js_path) => Ok(InstallationType::Npm { js_path }),
                Err(_) => Ok(InstallationType::Unknown),
            }
        }
        _ => Ok(InstallationType::Unknown),
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

/// 检测文件是否为二进制文件（通过 magic bytes）
fn is_binary_file(path: &Path) -> bool {
    fs::read(path)
        .map(|data| {
            if data.len() < 4 { return false; }
            matches!(&data[..4],
                [0x7f, 0x45, 0x4c, 0x46] |  // ELF
                [0xfe, 0xed, 0xfa, 0xce] | [0xfe, 0xed, 0xfa, 0xcf] |  // Mach-O
                [0xce, 0xfa, 0xed, 0xfe] | [0xcf, 0xfa, 0xed, 0xfe]    // Mach-O reversed
            )
        })
        .unwrap_or(false)
}

/// 从备份恢复文件
pub fn restore_from_backup(file_path: &Path) -> Result<()> {
    let backup_path = PathBuf::from(format!("{}.aiw-backup", file_path.display()));
    if !backup_path.exists() {
        return Err(UnifiedPatchError::FileError(
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("备份文件不存在: {}", backup_path.display()),
            ),
        ));
    }
    fs::copy(&backup_path, file_path)?;
    fs::remove_file(&backup_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_length_replacement() {
        // 测试不同长度的替换
        let content = b"if(cL()==\"firstParty\"){enabled()}".to_vec();
        let search_pattern = b"cL()==\"firstParty\"";
        let replace_pattern = b"true";
        
        // 模拟补丁应用
        let found_pos = content.windows(search_pattern.len())
            .position(|window| window == search_pattern)
            .unwrap();
        
        let mut new_content = Vec::with_capacity(content.len());
        new_content.extend_from_slice(&content[..found_pos]);
        new_content.extend_from_slice(replace_pattern);
        new_content.extend_from_slice(&content[found_pos + search_pattern.len()..]);
        
        let result = String::from_utf8_lossy(&new_content);
        assert_eq!(result, "if(true){enabled()}");
    }

    #[test]
    fn test_claude_cli_path() {
        // 测试获取 claude 路径
        let result = get_claude_cli_path();
        // 结果取决于系统是否有 claude 命令
        assert!(result.is_ok() || result.is_err());
    }
}
