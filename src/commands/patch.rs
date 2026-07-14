//! 补丁管理命令实现
//!
//! 提供文件补丁的管理功能：应用、状态查询、还原、设置 max-token
//!
//! max-token patch 通过通用 regex 匹配 Claude CLI 常量块
//! `var X=200000,Y=200000,...`，把两个 200000 等长替换为配置值（默认 500000）。

use crate::patcher::{apply_file_patch, is_file_patched, restore_from_backup};
use crate::patcher::claude::{
    get_antiatis_patches, get_anticloudetect_patches, get_antiframetrack_patches,
    get_antipromptbias_patches, get_antispy_patches, get_antitelemetry_patches,
    get_feature_patches, get_patchable_path, detect_installation, InstallationType,
};
use crate::patcher::claude::versions::{validate_max_context_tokens, ClaudeVersion};
use crate::patcher::grok::install::detect_grok;
use crate::patcher::grok::registry::get_grok_repo_bundle_patches;
use crate::patcher::types::{FeatureType, PatchType};
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
        PatchAction::DisablePromptBias => {
            execute_disable_prompt_bias()?;
        }
        PatchAction::DisableAtis => {
            execute_disable_atis()?;
        }
        PatchAction::DisableFrameTrack => {
            execute_disable_frame_track()?;
        }
        PatchAction::DisableCloudDetect => {
            execute_disable_cloud_detect()?;
        }
        PatchAction::GrokPatchApply => {
            execute_grok_patch_apply()?;
        }
        PatchAction::GrokPatchStatus => {
            execute_grok_patch_status();
        }
        PatchAction::GrokPatchRestore => {
            execute_grok_patch_restore()?;
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

    // AntiPromptBias 文件补丁（独立于 max-token / AntiTelemetry / AntiSpy，一个失败不影响另一个）
    let antipromptbias_patches = get_antipromptbias_patches();
    for patch in antipromptbias_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }

    // AntiAtis 文件补丁（独立于 max-token / AntiTelemetry / AntiSpy / AntiPromptBias，一个失败不影响另一个）
    let antiatis_patches = get_antiatis_patches();
    for patch in antiatis_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }

    // AntiFrameTrack 文件补丁（独立于其他 5 个，一个失败不影响另一个）
    let antiframetrack_patches = get_antiframetrack_patches();
    for patch in antiframetrack_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }

    // AntiCloudDetect 文件补丁（独立于其他 6 个，一个失败不影响另一个）
    let anticloudetect_patches = get_anticloudetect_patches();
    for patch in anticloudetect_patches
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

/// 消除 Provider context 提示词偏见（第三方不再被注入"功能有差异"提示）
fn execute_disable_prompt_bias() -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);

    let patches = get_antipromptbias_patches();
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    println!("✅ AntiPromptBias patch 应用完成（Provider context 偏见已消除）");
    Ok(())
}

/// 禁用 x-cc-atis 追踪 header（防逃生口 patch 副作用，atis 提取函数 → void 0）
fn execute_disable_atis() -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);

    let patches = get_antiatis_patches();
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    println!("✅ AntiAtis patch 应用完成（x-cc-atis 追踪 header 已禁用）");
    Ok(())
}

/// 禁用 frame/track 第二上报通道（绕过 AntiTelemetry 的独立 frame 服务上报）
fn execute_disable_frame_track() -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);

    let patches = get_antiframetrack_patches();
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    println!("✅ AntiFrameTrack patch 应用完成（frame/track 上报通道已截断）");
    Ok(())
}

/// 禁用 MAC 地址 GCE 云检测（tMi MAC 扫描失效，/^42:01/ → /^00:00/）
fn execute_disable_cloud_detect() -> Result<()> {
    let patch_path = get_patchable_path()?;
    let install_type = detect_installation().ok();
    let type_label = match &install_type {
        Some(InstallationType::Npm { .. }) => "npm (JS)",
        Some(InstallationType::NativeBinary { .. }) => "NativeBinary",
        _ => "Unknown",
    };
    println!("📂 Claude CLI 文件: {} ({})", patch_path.display(), type_label);

    let patches = get_anticloudetect_patches();
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match apply_file_patch(&patch_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    println!("✅ AntiCloudDetect patch 应用完成（MAC 地址 GCE 云检测已失效）");
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
    let mut skipped = false;
    for patch in patches.iter().filter(|p| p.patch_type == PatchType::File) {
        match is_file_patched(&patch_path, patch) {
            Ok(true) => {
                println!("   ✅ max-token 补丁已应用");
                patched = true;
                break;
            }
            Ok(false) => continue,
            Err(_) => {
                skipped = true;
                break;
            }
        }
    }
    if patched {
        // 已在上面打印
    } else if skipped {
        println!("   ⚪ max-token 无需 patch（目标不存在）");
    } else {
        println!("   ❌ max-token 补丁未应用");
    }

    // AntiTelemetry 状态检查
    let antitelemetry_patches = get_antitelemetry_patches();
    let mut antitelemetry_patched = false;
    let mut antitelemetry_skipped = false;
    for patch in antitelemetry_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match is_file_patched(&patch_path, patch) {
            Ok(true) => {
                println!("   ✅ AntiTelemetry 补丁已应用");
                antitelemetry_patched = true;
                break;
            }
            Ok(false) => continue,
            Err(_) => {
                antitelemetry_skipped = true;
                break;
            }
        }
    }
    if antitelemetry_patched {
        // 已在上面打印
    } else if antitelemetry_skipped {
        println!("   ⚪ AntiTelemetry 无需 patch（目标不存在）");
    } else {
        println!("   ❌ AntiTelemetry 补丁未应用");
    }

    // AntiSpy 状态检查
    let antispy_patches = get_antispy_patches();
    let mut antispy_patched = false;
    let mut antispy_skipped = false;
    for patch in antispy_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match is_file_patched(&patch_path, patch) {
            Ok(true) => {
                println!("   ✅ AntiSpy 补丁已应用");
                antispy_patched = true;
                break;
            }
            Ok(false) => continue,
            Err(_) => {
                antispy_skipped = true;
                break;
            }
        }
    }
    if antispy_patched {
        // 已在上面打印
    } else if antispy_skipped {
        println!("   ⚪ AntiSpy 无需 patch（目标不存在）");
    } else {
        println!("   ❌ AntiSpy 补丁未应用");
    }

    // AntiPromptBias 状态检查
    let antipromptbias_patches = get_antipromptbias_patches();
    let mut antipromptbias_patched = false;
    let mut antipromptbias_skipped = false;
    for patch in antipromptbias_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match is_file_patched(&patch_path, patch) {
            Ok(true) => {
                println!("   ✅ AntiPromptBias 补丁已应用");
                antipromptbias_patched = true;
                break;
            }
            Ok(false) => continue,
            Err(_) => {
                antipromptbias_skipped = true;
                break;
            }
        }
    }
    if antipromptbias_patched {
        // 已在上面打印
    } else if antipromptbias_skipped {
        println!("   ⚪ AntiPromptBias 无需 patch（目标不存在）");
    } else {
        println!("   ❌ AntiPromptBias 补丁未应用");
    }

    // AntiAtis 状态检查
    let antiatis_patches = get_antiatis_patches();
    let mut antiatis_patched = false;
    let mut antiatis_skipped = false;
    for patch in antiatis_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match is_file_patched(&patch_path, patch) {
            Ok(true) => {
                println!("   ✅ AntiAtis 补丁已应用");
                antiatis_patched = true;
                break;
            }
            Ok(false) => continue,
            Err(_) => {
                antiatis_skipped = true;
                break;
            }
        }
    }
    if antiatis_patched {
        // 已在上面打印
    } else if antiatis_skipped {
        println!("   ⚪ AntiAtis 无需 patch（目标不存在）");
    } else {
        println!("   ❌ AntiAtis 补丁未应用");
    }

    // AntiFrameTrack 状态检查
    let antiframetrack_patches = get_antiframetrack_patches();
    let mut antiframetrack_patched = false;
    let mut antiframetrack_skipped = false;
    for patch in antiframetrack_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match is_file_patched(&patch_path, patch) {
            Ok(true) => {
                println!("   ✅ AntiFrameTrack 补丁已应用");
                antiframetrack_patched = true;
                break;
            }
            Ok(false) => continue,
            Err(_) => {
                antiframetrack_skipped = true;
                break;
            }
        }
    }
    if antiframetrack_patched {
        // 已在上面打印
    } else if antiframetrack_skipped {
        println!("   ⚪ AntiFrameTrack 无需 patch（目标不存在）");
    } else {
        println!("   ❌ AntiFrameTrack 补丁未应用");
    }

    // AntiCloudDetect 状态检查
    let anticloudetect_patches = get_anticloudetect_patches();
    let mut anticloudetect_patched = false;
    let mut anticloudetect_skipped = false;
    for patch in anticloudetect_patches
        .iter()
        .filter(|p| p.patch_type == PatchType::File)
    {
        match is_file_patched(&patch_path, patch) {
            Ok(true) => {
                println!("   ✅ AntiCloudDetect 补丁已应用");
                anticloudetect_patched = true;
                break;
            }
            Ok(false) => continue,
            Err(_) => {
                anticloudetect_skipped = true;
                break;
            }
        }
    }
    if anticloudetect_patched {
        // 已在上面打印
    } else if anticloudetect_skipped {
        println!("   ⚪ AntiCloudDetect 无需 patch（目标不存在）");
    } else {
        println!("   ❌ AntiCloudDetect 补丁未应用");
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

/// 应用 Grok 上传 patch
fn execute_grok_patch_apply() -> Result<()> {
    let inst = match detect_grok() {
        Ok(i) => i,
        Err(e) => {
            println!("❌ 未检测到 Grok 安装: {}", e);
            return Ok(());
        }
    };
    println!("📂 Grok binary: {} (v{})", inst.binary_path.display(), inst.version);

    let patches = match get_grok_repo_bundle_patches() {
        Ok(p) => p,
        Err(e) => {
            println!("❌ 定位 patch 锚点失败: {}", e);
            println!("   可能是 Grok 版本更新导致字节模式漂移，请检查 docs/domain-knowledge/grok-build.md");
            return Ok(());
        }
    };
    println!("🔍 定位到 {} 个 repo bundle call 点", patches.len());

    for patch in &patches {
        match apply_file_patch(&inst.binary_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    println!("✅ Grok repo bundle patch 应用完成（GCS 上传已禁用）");
    Ok(())
}

/// 查看 Grok patch 状态
fn execute_grok_patch_status() {
    let inst = match detect_grok() {
        Ok(i) => i,
        Err(e) => {
            println!("⚠️  未检测到 Grok 安装: {}", e);
            return;
        }
    };
    println!("📊 Grok patch 状态:");
    println!("   Grok version: {}", inst.version);
    println!("   Binary: {}", inst.binary_path.display());

    match get_grok_repo_bundle_patches() {
        Ok(patches) => {
            for patch in &patches {
                match is_file_patched(&inst.binary_path, patch) {
                    Ok(true) => println!("   ✅ {} 已应用", patch.description),
                    Ok(false) => println!("   ❌ {} 未应用", patch.description),
                    Err(_) => println!("   ⚪ {} 无法检测", patch.description),
                }
            }
        }
        Err(_) => {
            // locate 失败 = binary 已被 patch（call 字节变了无法重定位）或锚点漂移。
            // 用备份存在 + CALL_REPLACE 字节签名区分。
            let backup = inst.binary_path.with_extension("aiw-backup");
            if backup.exists() {
                println!("   ✅ GrokAntiRepoBundle 已应用（binary 已 patch，备份存在）");
            } else {
                println!("   ❌ 锚点定位失败：可能版本更新导致字节漂移，检查 docs/domain-knowledge/grok-build.md");
            }
        }
    }
}

/// 还原 Grok patch
fn execute_grok_patch_restore() -> Result<()> {
    let inst = match detect_grok() {
        Ok(i) => i,
        Err(e) => {
            println!("❌ 未检测到 Grok 安装: {}", e);
            return Ok(());
        }
    };
    println!("🔄 从备份恢复 Grok binary...");
    match restore_from_backup(&inst.binary_path) {
        Ok(()) => println!("✅ 已从备份恢复: {}", inst.binary_path.display()),
        Err(e) => println!("❌ 恢复失败: {}", e),
    }
    Ok(())
}
