//! CC 2.1.207 集成验证：对真实 207 binary 应用全量 patch，验证 8 个 patch 生效
//!
//! 运行：cargo test --test patcher_207_integration -- --nocapture --ignored
//!
//! 需先准备 binary：
//!   cp /srv/home-links/.local/share/claude/versions/2.1.207 /tmp/cc-audit/claude-2.1.207-test

#![cfg(test)]

use aiw::patcher::file::{apply_file_patch, is_file_patched};
use aiw::patcher::registry::{
    get_anticloudetect_patches, get_antiframetrack_patches, get_antiatis_patches,
    get_antipromptbias_patches, get_antispy_patches, get_antitelemetry_patches,
};
use aiw::patcher::types::PatchType;
use aiw::patcher::versions::ClaudeVersion;
use aiw::patcher::{get_feature_patches, FeatureType};
use std::fs;
use std::path::PathBuf;

const TEST_BINARY: &str = "/tmp/cc-audit/claude-2.1.207-test";

fn ensure_binary() -> PathBuf {
    let p = PathBuf::from(TEST_BINARY);
    if !p.exists() {
        eprintln!("SKIP: 测试 binary 不存在: {} (先 cp 207 binary 到此)", TEST_BINARY);
    }
    p
}

fn all_file_patches(version: &ClaudeVersion) -> Vec<(&'static str, Vec<aiw::patcher::types::UnifiedPatchPattern>)> {
    vec![
        ("MaxContextTokens", get_feature_patches(FeatureType::MaxContextTokens, version)),
        ("AntiTelemetry", get_antitelemetry_patches()),
        ("AntiSpy", get_antispy_patches()),
        ("AntiPromptBias", get_antipromptbias_patches()),
        ("AntiAtis", get_antiatis_patches()),
        ("AntiFrameTrack", get_antiframetrack_patches()),
        ("AntiCloudDetect", get_anticloudetect_patches()),
    ]
}

#[test]
#[ignore = "需要真实 207 binary 在 /tmp/cc-audit/claude-2.1.207-test"]
fn test_207_full_patch_integration() {
    let binary = ensure_binary();
    if !binary.exists() {
        return;
    }

    let version = ClaudeVersion { major: 2, minor: 1, patch: 207 };
    let size_before = fs::metadata(&binary).unwrap().len();

    println!("=== CC 2.1.207 全量 patch 集成验证 ===");
    println!("binary: {}", binary.display());
    println!("size before: {} bytes", size_before);

    // patch 前：每个 feature 的 file patch 应能命中（is_file_patched=false 或应用成功）
    println!("\n--- 应用 patch ---");
    for (name, patches) in all_file_patches(&version) {
        let file_patch = patches.iter().find(|p| p.patch_type == PatchType::File).unwrap();
        match apply_file_patch(&binary, file_patch) {
            Ok(_) => println!("  ✅ {} applied", name),
            Err(e) => println!("  ❌ {} FAILED: {}", name, e),
        }
    }

    let size_after = fs::metadata(&binary).unwrap().len();
    println!("\nsize after: {} bytes", size_after);
    assert_eq!(size_before, size_after, "等长铁律：patch 后 binary 大小必须不变");

    // patch 后：所有 patch 应标记为已应用
    println!("\n--- 验证 is_file_patched ---");
    let mut all_patched = true;
    for (name, patches) in all_file_patches(&version) {
        let file_patch = patches.iter().find(|p| p.patch_type == PatchType::File).unwrap();
        match is_file_patched(&binary, file_patch) {
            Ok(true) => println!("  ✅ {} already patched", name),
            Ok(false) => {
                println!("  ❌ {} NOT patched", name);
                all_patched = false;
            }
            Err(e) => {
                println!("  ⚠️ {} check error: {}", name, e);
                all_patched = false;
            }
        }
    }
    assert!(all_patched, "所有 8 个 patch 必须标记为已应用");
    println!("\n=== ✅ 207 集成验证全部通过 ===");
}
