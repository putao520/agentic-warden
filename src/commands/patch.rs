//! 补丁管理命令实现
//!
//! 提供文件补丁的管理功能：列表、应用、状态查询、还原

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
        PatchAction::List { verbose } => {
            execute_list_patches(verbose);
        }
        PatchAction::Apply { feature } => {
            execute_apply_patch(&feature)?;
        }
        PatchAction::Status { feature } => {
            execute_patch_status(&feature);
        }
        PatchAction::Restore { feature } => {
            execute_restore_patch(&feature)?;
        }
    }
    Ok(())
}

/// 列出所有可用的补丁
fn execute_list_patches(verbose: bool) {
    println!("📋 可用的补丁:");
    println!();
    
    let features = [
        FeatureType::ToolSearch,
        FeatureType::UltraThink,
        FeatureType::WebSearch,
        FeatureType::PersistentMemory,
    ];

    // 获取当前 Claude 版本
    let version = get_claude_version();
    
    for feature in features {
        let patches = get_feature_patches(feature, &version);
        
        if patches.is_empty() {
            println!("  ❌ {:?} - 无可用补丁", feature);
            continue;
        }

        println!("  ✅ {}", feature.description());
        println!("     简称: {}", feature.short_name());
        
        if verbose {
            println!("     可用补丁:");
            for patch in &patches {
                println!("       - {} ({:?})", patch.description, patch.patch_type);
            }
        }
        println!();
    }
    
    // 显示补丁类型说明
    println!("补丁类型:");
    println!("  • File   - 文件补丁（持久化，修改磁盘文件）");
    println!("  • Memory - 内存补丁（运行时，每次启动自动应用）");
}

/// 应用文件补丁
fn execute_apply_patch(feature_name: &str) -> Result<()> {
    let version = get_claude_version();
    
    let features = if feature_name == "all" {
        vec![
            FeatureType::ToolSearch,
            FeatureType::UltraThink,
            FeatureType::WebSearch,
        ]
    } else {
        vec![parse_feature_type(feature_name)?]
    };

    // 获取可补丁的文件路径
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);

    for feature in features {
        println!("🔧 应用补丁: {}", feature.description());
        
        let patches = get_feature_patches(feature, &version);
        let file_patches: Vec<_> = patches
            .iter()
            .filter(|p| p.patch_type == PatchType::File)
            .collect();

        if file_patches.is_empty() {
            println!("   ⚠️  无文件补丁可用");
            continue;
        }

        for patch in file_patches {
            match apply_file_patch(&patch_path, patch) {
                Ok(_) => println!("   ✅ {}", patch.description),
                Err(e) => {
                    println!("   ❌ 失败: {}", e);
                    // 继续尝试其他补丁
                }
            }
        }
    }

    println!("✅ 补丁应用完成!");
    Ok(())
}

/// 查看补丁状态
fn execute_patch_status(feature_name: &Option<String>) {
    let version = get_claude_version();
    
    let features = if let Some(name) = feature_name {
        vec![parse_feature_type(name).unwrap()]
    } else {
        vec![
            FeatureType::ToolSearch,
            FeatureType::UltraThink,
            FeatureType::WebSearch,
        ]
    };

    // 获取可补丁的文件路径
    let patch_path = match get_patchable_path() {
        Ok(path) => path,
        Err(_) => {
            println!("⚠️  未找到可补丁的 Claude CLI");
            println!("   支持 npm 安装和本地二进制安装");
            return;
        }
    };

    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };

    println!("📊 补丁状态:");
    println!("   Claude 版本: {}", format_version(&version));
    println!("   安装类型: {}", type_label);
    println!("   文件: {}", patch_path.display());
    println!();

    for feature in features {
        println!("   {}", feature.description());
        
        let patches = get_feature_patches(feature, &version);
        let file_patches: Vec<_> = patches
            .iter()
            .filter(|p| p.patch_type == PatchType::File)
            .collect();

        if file_patches.is_empty() {
            println!("     无文件补丁");
            continue;
        }

        for patch in file_patches {
            match is_file_patched(&patch_path, patch) {
                Ok(true) => println!("     ✅ 已补丁: {}", patch.description),
                Ok(false) => println!("     ❌ 未补丁: {}", patch.description),
                Err(e) => println!("     ⚠️  检查失败: {}", e),
            }
        }
    }
}

/// 还原文件补丁
fn execute_restore_patch(feature_name: &str) -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();

    match &install_type {
        Some(InstallationType::NativeBinary { .. }) => {
            // 从备份恢复
            println!("🔄 从备份恢复 NativeBinary...");
            match restore_from_backup(&patch_path) {
                Ok(()) => println!("✅ 已从备份恢复: {}", patch_path.display()),
                Err(e) => println!("❌ 恢复失败: {}", e),
            }
        }
        Some(InstallationType::Npm { .. }) => {
            println!("ℹ️  npm 安装版本请重新安装以还原:");
            println!("   npm install -g @anthropic-ai/claude-code");
        }
        _ => {
            println!("⚠️  无法识别安装类型，无法还原");
        }
    }
    Ok(())
}

/// 解析功能类型
fn parse_feature_type(name: &str) -> Result<FeatureType> {
    match name.to_lowercase().as_str() {
        "toolsearch" => Ok(FeatureType::ToolSearch),
        "ultrathink" => Ok(FeatureType::UltraThink),
        "websearch" => Ok(FeatureType::WebSearch),
        "agentteams" => Ok(FeatureType::AgentTeams),
        "persistent" | "persistentmemory" => Ok(FeatureType::PersistentMemory),
        _ => Err(anyhow::anyhow!("未知的功能: {}", name)),
    }
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
