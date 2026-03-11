//! 补丁管理命令实现
//!
//! 提供文件补丁的管理功能：应用、状态查询、还原

use crate::patcher::{
    get_patchable_path, detect_installation, is_file_patched, apply_file_patch,
    restore_from_backup, InstallationType,
    types::{FeatureType, PatchType},
    registry::get_feature_patches,
    versions::ClaudeVersion,
};
use crate::commands::parser::PatchAction;
use anyhow::Result;

/// 执行补丁命令
pub async fn execute_patch_command(action: PatchAction) -> Result<()> {
    match action {
        PatchAction::Apply => {
            execute_apply_patch()?;
        }
        PatchAction::Status => {
            execute_patch_status();
        }
        PatchAction::Restore => {
            execute_restore_patch()?;
        }
    }
    Ok(())
}

/// 应用文件补丁
fn execute_apply_patch() -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);
    
    let version = get_claude_version();

    if !version.is_supported() {
        println!("❌ Claude CLI {}.{}.{} is not in supported versions ({}).",
            version.major, version.minor, version.patch,
            ClaudeVersion::supported_versions_str());
        println!("   Patches cannot be applied to this version.");
        return Ok(());
    }

    let patches = get_feature_patches(FeatureType::ToolSearch, &version);
    
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    
    println!("✅ 补丁应用完成!");
    Ok(())
}

/// 查看补丁状态
fn execute_patch_status() {
    let patch_path = match get_patchable_path() {
        Ok(path) => path,
        Err(_) => {
            println!("⚠️  未找到可补丁的 Claude CLI");
            return;
        }
    };

    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };

    let version = get_claude_version();
    println!("📊 补丁状态:");
    println!("   Claude version: {}", format_version(&version));
    println!("   Supported versions: {}", ClaudeVersion::supported_versions_str());
    println!("   Version supported: {}", if version.is_supported() { "Yes" } else { "No" });
    println!("   Install type: {}", type_label);
    println!("   File: {}", patch_path.display());

    let patches = get_feature_patches(FeatureType::ToolSearch, &version);

    let mut patched = false;
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match is_file_patched(&patch_path, patch) {
            Ok(true) => {
                println!("   ✅ 补丁已应用");
                patched = true;
                break;
            }
            _ => {}
        }
    }
    if !patched {
        println!("   ❌ 补丁未应用");
    }
}

/// 还原文件补丁
fn execute_restore_patch() -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();

    match &install_type {
        Some(InstallationType::NativeBinary { .. }) => {
            println!("🔄 从备份恢复...");
            match restore_from_backup(&patch_path) {
                Ok(()) => println!("✅ 已从备份恢复: {}", patch_path.display()),
                Err(e) => println!("❌ 恢复失败: {}", e),
            }
        }
        Some(InstallationType::Npm { .. }) => {
            println!("ℹ️  npm 安装版本请重新安装:");
            println!("   npm install -g @anthropic-ai/claude-code");
        }
        _ => {
            println!("⚠️  无法识别安装类型，无法还原");
        }
    }
    Ok(())
}

/// 获取 Claude 版本
fn get_claude_version() -> ClaudeVersion {
    use std::process::Command;
    
    let output = Command::new("claude")
        .arg("--version")
        .output()
        .unwrap_or_else(|_| {
            // 默认使用当前版本
            Command::new("sh")
                .arg("-c")
                .arg("echo '2.1.72'")
                .output()
                .unwrap()
        });

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = version_str
        .split_whitespace()
        .next()
        .unwrap_or("2.1.72");
    
    ClaudeVersion::from_string(version)
        .unwrap_or(ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 72,
        })
}

/// 格式化版本号
fn format_version(version: &ClaudeVersion) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}
