//! 补丁管理命令实现
//!
//! 提供文件补丁的管理功能：列表、应用、状态查询、还原

use crate::patcher::{
    get_claude_js_path, is_file_patched, apply_file_patch,
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

    // 获取 Claude CLI 文件路径
    let js_path = get_claude_js_path()?;
    println!("📂 Claude CLI 文件: {}", js_path.display());

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
            match apply_file_patch(&js_path, patch) {
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

    // 获取 Claude CLI 文件路径
    let js_path = match get_claude_js_path() {
        Ok(path) => path,
        Err(_) => {
            println!("⚠️  未找到 npm 安装的 Claude CLI");
            println!("   文件补丁仅适用于 npm 安装版本");
            println!("   cargo 安装版本会自动使用内存补丁");
            return;
        }
    };

    println!("📊 补丁状态:");
    println!("   Claude 版本: {}", format_version(&version));
    println!("   文件: {}", js_path.display());
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
            match is_file_patched(&js_path, patch) {
                Ok(true) => println!("     ✅ 已补丁: {}", patch.description),
                Ok(false) => println!("     ❌ 未补丁: {}", patch.description),
                Err(e) => println!("     ⚠️  检查失败: {}", e),
            }
        }
    }
}

/// 还原文件补丁
fn execute_restore_patch(feature_name: &str) -> Result<()> {
    println!("⚠️  还原功能尚未实现");
    println!("   如需还原，请重新安装 Claude CLI:");
    println!("   npm install -g @anthropic-ai/claude-code");
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
