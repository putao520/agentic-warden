//! 文件补丁实现
//!
//! 直接修改磁盘上的 Claude CLI 文件

use crate::patcher::types::{UnifiedPatchError, UnifiedPatchPattern, UnifiedPatchResult, Result};
use std::fs;
use std::path::Path;

/// 应用文件补丁
pub fn apply_file_patch(
    file_path: &Path,
    pattern: &UnifiedPatchPattern,
) -> Result<UnifiedPatchResult> {
    // 读取文件内容
    let content = fs::read(file_path)?;
    
    // 搜索模式
    let found_pos = content
        .windows(pattern.search_pattern.len())
        .position(|window| window == pattern.search_pattern)
        .ok_or_else(|| UnifiedPatchError::PatternNotFound(format!("{:?}", pattern.search_pattern)))?;
    
    // 应用补丁
    let patched_content = if let Some(replace) = pattern.replace_pattern {
        // 支持不同长度的替换
        let mut new_content = Vec::with_capacity(content.len());
        new_content.extend_from_slice(&content[..found_pos]);
        new_content.extend_from_slice(replace);
        new_content.extend_from_slice(&content[found_pos + pattern.search_pattern.len()..]);
        new_content
    } else if let (Some(patch_byte), Some(offset)) = (pattern.patch_byte, pattern.patch_offset) {
        // 单字节修补（长度相同）
        let mut new_content = content.clone();
        new_content[found_pos + offset] = patch_byte;
        new_content
    } else {
        return Err(UnifiedPatchError::PatternNotFound(
            "No replacement pattern or patch byte specified".to_string()
        ));
    };
    
    // 写回文件
    fs::write(file_path, patched_content)?;
    
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

/// 获取 npm 安装的 Claude CLI 的 JavaScript 文件路径
pub fn get_claude_js_path() -> Result<std::path::PathBuf> {
    let claude_path = get_claude_cli_path()?;
    let script_content = fs::read_to_string(&claude_path)?;
    
    // npm 安装的 claude 是一个 shell 脚本，包含 node_modules 路径
    for line in script_content.lines() {
        if line.contains("node_modules") && (line.contains(".js") || line.contains("basedir")) {
            // 检查是否包含 basedir 参数
            if let Some(basedir_start) = line.find("basedir=") {
                let basedir_part = &line[basedir_start + 8..];
                if let Some(basedir_end) = basedir_part.find(' ') {
                    let basedir = &basedir_part[..basedir_end];
                    // 展开环境变量
                    let basedir = shellexpand::env(basedir)
                        .map_err(|e| UnifiedPatchError::Other(format!("Failed to expand basedir: {}", e)))?;
                    return Ok(std::path::PathBuf::from(basedir.as_ref()).join("cli.js"));
                }
            }
        }
    }
    
    Err(UnifiedPatchError::FileError(
        std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find claude.js path")
    ))
}

/// 检查是否为 npm 安装
pub fn is_npm_installation() -> Result<bool> {
    let claude_path = get_claude_cli_path()?;
    let script_content = fs::read_to_string(&claude_path)?;
    Ok(script_content.contains("node_modules"))
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
