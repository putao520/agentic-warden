//! Grok patch pattern 生成
//!
//! 为 3 个 Grok feature 生成 UnifiedPatchPattern。repo bundle 已定案
//! （call→31 c0 48 89 07），deploy/trace 实现阶段定位。

use crate::patcher::grok::install::{detect_grok, get_grok_binary_path};
use crate::patcher::grok::targets::{locate_repo_bundle_call_sites_versioned, CALL_REPLACE};
use crate::patcher::types::{FeatureType, PatchType, UnifiedPatchPattern, UnifiedPatchError};
use std::borrow::Cow;

/// 生成 repo bundle 上传 patch（2 个 call 点 → 31 c0 48 89 07）
///
/// 运行时读 binary + 探测版本，用版本表区分 GCS dispatcher（0.2.101+ 候选同构
/// 需版本表），定位 2 个 call 点生成等长替换 pattern。
pub fn get_grok_repo_bundle_patches() -> Result<Vec<UnifiedPatchPattern>, UnifiedPatchError> {
    let binary_path = get_grok_binary_path()?;
    let binary = std::fs::read(&binary_path)?;
    // 探测版本传给 locate（版本表区分 GCS）
    let version = detect_grok()
        .ok()
        .map(|i| format!("{}", i.version));
    let sites = locate_repo_bundle_call_sites_versioned(&binary, version.as_deref().unwrap_or(""))?;
    let mut patches = Vec::with_capacity(sites.len());
    for off in sites {
        // search: call 指令的 5 字节（e8 + 4 字节 rel32，从 binary 实际读取）
        let search: Vec<u8> = binary[off..off + 5].to_vec();
        patches.push(UnifiedPatchPattern {
            feature: FeatureType::GrokAntiRepoBundle,
            patch_type: PatchType::File,
            search_pattern: Cow::Owned(search),
            replace_pattern: Some(Cow::Borrowed(CALL_REPLACE)),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Owned(format!(
                "GrokAntiRepoBundle: GCS upload call @ {:#x} → xor+mov (no-op upload)",
                off
            )),
            use_regex: false,
            regex_replace_values: None,
            dynamic_replace: None,
        });
    }
    Ok(patches)
}

/// deploy upload patch（实现阶段定位，暂返回空 + 诊断）
pub fn get_grok_deploy_upload_patches() -> Result<Vec<UnifiedPatchPattern>, UnifiedPatchError> {
    // TODO Task 8: 定位 [deploy_app] starting upload build 的 call 点
    Ok(vec![])
}

/// trace upload patch（实现阶段定位，暂返回空 + 诊断）
pub fn get_grok_trace_upload_patches() -> Result<Vec<UnifiedPatchPattern>, UnifiedPatchError> {
    // TODO Task 8: 定位 upload session trace 的 call 点
    Ok(vec![])
}
