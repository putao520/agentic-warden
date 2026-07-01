//! 补丁管理命令实现
//!
//! 提供文件补丁的管理功能：应用、状态查询、还原、设置 max-token
//!
//! max-token patch 通过通用 regex 匹配 Claude CLI 常量块
//! `var X=200000,Y=200000,...`，把两个 200000 等长替换为配置值（默认 500000）。

use crate::patcher::{
    apply_file_patch, detect_installation, get_patchable_path, is_file_patched,
    restore_from_backup, InstallationType,
    registry::{get_antispy_patches, get_antitelemetry_patches, get_feature_patches},
    types::{FeatureType, PatchType},
    versions::{validate_max_context_tokens, ClaudeVersion},
};
use crate::commands::parser::PatchAction;
use crate::config::PatchConfig;
use anyhow::Result;

/// 执行补丁命令
pub async fn execute_patch_command(action: PatchAction) -> Result<()> {
    match action {
        PatchAction::Apply {
            max_context_tokens,
            auto_compact_window,
        } => {
            execute_apply_patch(max_context_tokens, auto_compact_window)?;
        }
        PatchAction::Status => {
            execute_patch_status();
        }
        PatchAction::Restore => {
            execute_restore_patch()?;
        }
        PatchAction::SetMaxTokens {
            max_context_tokens,
            auto_compact_window,
        } => {
            execute_set_max_tokens(max_context_tokens, auto_compact_window)?;
        }
        PatchAction::DisableTelemetry => {
            execute_disable_telemetry()?;
        }
        PatchAction::DisableSpy => {
            execute_disable_spy()?;
        }
    }
    Ok(())
}

/// 应用文件补丁（max-token + anti-telemetry + anti-spy）
///
/// CLI 参数覆盖配置文件值并持久化（与 set-max-tokens 行为一致）：
/// - 只指定 `--max-context-tokens` 时，`auto_compact_window` 跟齐
/// - 两个参数都指定时，分别设置
/// - 都不指定时，使用配置文件默认值（或 500000）
fn execute_apply_patch(
    max_context_tokens: Option<u32>,
    auto_compact_window: Option<u32>,
) -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);

    // 读配置默认值（或默认 500000），CLI 参数覆盖
    let mut cfg = PatchConfig::load().unwrap_or_default();
    if let Some(mt) = max_context_tokens {
        validate_max_context_tokens(mt).map_err(|e| anyhow::anyhow!(e))?;
        cfg.max_context_tokens = mt;
        // 只指定 max 时 auto 跟齐
        if auto_compact_window.is_none() {
            cfg.auto_compact_window = mt;
        }
    }
    if let Some(ac) = auto_compact_window {
        validate_max_context_tokens(ac).map_err(|e| anyhow::anyhow!(e))?;
        cfg.auto_compact_window = ac;
    }
    // 持久化用户选择（与 set-max-tokens 一致）
    if max_context_tokens.is_some() || auto_compact_window.is_some() {
        let _ = cfg.save();
    }
    println!(
        "🔧 max_context_tokens={}, auto_compact_window={}",
        cfg.max_context_tokens, cfg.auto_compact_window
    );

    let version = get_claude_version();

    // max-token 文件补丁
    let max_token_patches = get_feature_patches(FeatureType::MaxContextTokens, &version);
    for patch in max_token_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }

    // AntiTelemetry 文件补丁（独立于 max-token，一个失败不影响另一个）
    let antitelemetry_patches = get_antitelemetry_patches();
    for patch in antitelemetry_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }

    // AntiSpy 文件补丁（独立于 max-token / AntiTelemetry，一个失败不影响另一个）
    let antispy_patches = get_antispy_patches();
    for patch in antispy_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }

    println!("✅ 补丁应用完成!");
    Ok(())
}

/// 禁用 CC 客户端上报（截断 event_logging 端点）
fn execute_disable_telemetry() -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);

    let patches = get_antitelemetry_patches();
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    println!("✅ AntiTelemetry patch 应用完成（上报已截断）");
    Ok(())
}

/// 禁用 CC 本地识别（时区+中转站失明）
fn execute_disable_spy() -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);

    let patches = get_antispy_patches();
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    println!("✅ AntiSpy patch 应用完成（时区+中转站识别已失明）");
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
    let cfg = PatchConfig::load().unwrap_or_default();
    println!("📊 补丁状态:");
    println!("   Claude version: {}", format_version(&version));
    println!("   Install type: {}", type_label);
    println!("   File: {}", patch_path.display());
    println!(
        "   Config: max_context_tokens={}, auto_compact_window={}",
        cfg.max_context_tokens, cfg.auto_compact_window
    );

    let patches = get_feature_patches(FeatureType::MaxContextTokens, &version);

    let mut patched = false;
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        if let Ok(true) = is_file_patched(&patch_path, patch) {
            println!("   ✅ max-token 补丁已应用");
            patched = true;
            break;
        }
    }
    if !patched {
        println!("   ❌ max-token 补丁未应用");
    }

    // AntiTelemetry 状态检查
    let antitelemetry_patches = get_antitelemetry_patches();
    let mut antitelemetry_patched = false;
    for patch in antitelemetry_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        if let Ok(true) = is_file_patched(&patch_path, patch) {
            println!("   ✅ AntiTelemetry 补丁已应用");
            antitelemetry_patched = true;
            break;
        }
    }
    if !antitelemetry_patched {
        println!("   ❌ AntiTelemetry 补丁未应用");
    }

    // AntiSpy 状态检查
    let antispy_patches = get_antispy_patches();
    let mut antispy_patched = false;
    for patch in antispy_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        if let Ok(true) = is_file_patched(&patch_path, patch) {
            println!("   ✅ AntiSpy 补丁已应用");
            antispy_patched = true;
            break;
        }
    }
    if !antispy_patched {
        println!("   ❌ AntiSpy 补丁未应用");
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

/// 设置 max-token patch（持久化配置 + 应用文件补丁）
fn execute_set_max_tokens(
    max_context_tokens: Option<u32>,
    auto_compact_window: Option<u32>,
) -> Result<()> {
    // 读现有配置（或默认），CLI 参数覆盖
    let mut cfg = PatchConfig::load().unwrap_or_default();
    if let Some(v) = max_context_tokens {
        validate_max_context_tokens(v).map_err(|e| anyhow::anyhow!(e))?;
        cfg.max_context_tokens = v;
    }
    if let Some(v) = auto_compact_window {
        validate_max_context_tokens(v).map_err(|e| anyhow::anyhow!(e))?;
        cfg.auto_compact_window = v;
    }
    // 若只指定了 max_context_tokens，auto_compact 默认跟齐
    if max_context_tokens.is_some() && auto_compact_window.is_none() {
        cfg.auto_compact_window = cfg.max_context_tokens;
    }

    cfg.save()?;
    println!(
        "💾 已保存 patch 配置: max_context_tokens={}, auto_compact_window={}",
        cfg.max_context_tokens, cfg.auto_compact_window
    );

    // 立即应用文件补丁（配置已持久化，传 None 用配置值）
    execute_apply_patch(None, None)
}

/// 获取 Claude 版本
fn get_claude_version() -> ClaudeVersion {
    use std::process::Command;

    let output = Command::new("claude")
        .arg("--version")
        .output()
        .unwrap_or_else(|_| {
            // 默认使用当前版本
            Command::new("sh").arg("-c").arg("echo '2.1.195'").output().unwrap()
        });

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = version_str.split_whitespace().next().unwrap_or("2.1.195");

    ClaudeVersion::from_string(version).unwrap_or(ClaudeVersion {
        major: 2,
        minor: 1,
        patch: 195,
    })
}

/// 格式化版本号
fn format_version(version: &ClaudeVersion) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}
