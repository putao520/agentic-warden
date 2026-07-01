//! Patch registry - generates UnifiedPatchPattern for MaxContextTokens patch

use crate::patcher::types::{FeatureType, PatchType, UnifiedPatchPattern};
use crate::patcher::versions::{
    validate_max_context_tokens, ClaudeVersion, MAX_CONTEXT_TOKENS_SEARCH_REGEX,
};
use std::borrow::Cow;

/// Get patches for a feature based on version signature
///
/// 目前支持 MaxContextTokens 与 AntiTelemetry 两种功能；保留 version 参数
/// 以兼容调用方签名（regex/字面量均不依赖版本签名）。
pub fn get_feature_patches(
    feature: FeatureType,
    version: &ClaudeVersion,
) -> Vec<UnifiedPatchPattern> {
    match feature {
        FeatureType::MaxContextTokens => {
            get_max_context_tokens_patches(version, 500000, 500000)
        }
        FeatureType::AntiTelemetry => get_antitelemetry_patches(),
    }
}

/// 生成 max-token patch 模式
///
/// 通过通用 regex 匹配 Claude CLI 的常量块
/// `var X=200000,Y=200000,Z=20000,W=32000,Q=128000;`，把两个 200000
/// 等长替换为目标值。变量名因 minification 跨版本可能变化，故用 regex 通配。
///
/// # 参数
/// - `version`: Claude 版本（保留参数，regex 不依赖版本签名）
/// - `max_tokens`: 目标默认上下文窗口值（6 位数，100000~999999）
/// - `auto_compact`: autoCompact 阈值（6 位数，通常等于 max_tokens）
///
/// # Panics
/// 如果 max_tokens 或 auto_compact 不是 6 位十进制数则 panic（调用方应先校验）。
pub fn get_max_context_tokens_patches(
    _version: &ClaudeVersion,
    max_tokens: u32,
    auto_compact: u32,
) -> Vec<UnifiedPatchPattern> {
    validate_max_context_tokens(max_tokens).expect("invalid max_tokens");
    validate_max_context_tokens(auto_compact).expect("invalid auto_compact");

    let regex_bytes: Cow<'static, [u8]> =
        Cow::Borrowed(MAX_CONTEXT_TOKENS_SEARCH_REGEX.as_bytes());

    vec![
        // 文件补丁：regex 匹配 + 动态替换
        UnifiedPatchPattern {
            feature: FeatureType::MaxContextTokens,
            patch_type: PatchType::File,
            search_pattern: regex_bytes.clone(),
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "MaxContextTokens file patch: regex match + equal-length numeric replace",
            ),
            use_regex: true,
            regex_replace_values: Some(vec![max_tokens, auto_compact]),
        },
        // 内存补丁：regex 匹配 + 动态替换
        UnifiedPatchPattern {
            feature: FeatureType::MaxContextTokens,
            patch_type: PatchType::Memory,
            search_pattern: regex_bytes,
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "MaxContextTokens memory patch: regex match + equal-length numeric replace",
            ),
            use_regex: true,
            regex_replace_values: Some(vec![max_tokens, auto_compact]),
        },
    ]
}

/// 生成 AntiTelemetry patch 模式
///
/// 截断 CC 事件上报：`/api/event_logging/v2/batch` → `/api/event_logging/v2/xxxxx`
/// 等长 27 字节字面量替换，让端点 404，上报静默失败。跨版本稳定（API 路径字面量）。
///
/// 返回文件补丁与内存补丁各一份，内存补丁同样用 `replace_pattern` 整段字面量
/// 替换（不依赖 patch_byte/patch_offset）。
pub fn get_antitelemetry_patches() -> Vec<UnifiedPatchPattern> {
    vec![
        UnifiedPatchPattern {
            feature: FeatureType::AntiTelemetry,
            patch_type: PatchType::File,
            search_pattern: Cow::Borrowed(b"/api/event_logging/v2/batch"),
            replace_pattern: Some(Cow::Borrowed(b"/api/event_logging/v2/xxxxx")),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiTelemetry file patch: event_logging endpoint -> 404",
            ),
            use_regex: false,
            regex_replace_values: None,
        },
        UnifiedPatchPattern {
            feature: FeatureType::AntiTelemetry,
            patch_type: PatchType::Memory,
            search_pattern: Cow::Borrowed(b"/api/event_logging/v2/batch"),
            replace_pattern: Some(Cow::Borrowed(b"/api/event_logging/v2/xxxxx")),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiTelemetry memory patch: event_logging endpoint -> 404",
            ),
            use_regex: false,
            regex_replace_values: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_context_tokens_patches_structure() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 195,
        };
        let patches = get_max_context_tokens_patches(&version, 500000, 500000);
        assert_eq!(patches.len(), 2); // 1 file + 1 memory
        assert_eq!(patches[0].patch_type, PatchType::File);
        assert_eq!(patches[1].patch_type, PatchType::Memory);

        for p in &patches {
            assert!(p.use_regex);
            assert!(p.replace_pattern.is_none());
            assert_eq!(p.regex_replace_values, Some(vec![500000, 500000]));
            // search_pattern 存的是 regex 字符串字节
            let s = std::str::from_utf8(p.search_pattern.as_ref()).unwrap();
            assert!(s.contains("200000"));
        }
    }

    #[test]
    #[should_panic(expected = "invalid max_tokens")]
    fn test_max_context_tokens_invalid_max_tokens() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 195,
        };
        get_max_context_tokens_patches(&version, 99999, 500000);
    }

    #[test]
    #[should_panic(expected = "invalid auto_compact")]
    fn test_max_context_tokens_invalid_auto_compact() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 195,
        };
        get_max_context_tokens_patches(&version, 500000, 1000000);
    }

    #[test]
    fn test_max_context_tokens_via_get_feature_patches() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 195,
        };
        let patches = get_feature_patches(FeatureType::MaxContextTokens, &version);
        assert_eq!(patches.len(), 2);
        assert!(patches.iter().all(|p| p.use_regex));
    }

    #[test]
    fn test_antitelemetry_patches_structure() {
        let patches = get_antitelemetry_patches();
        assert_eq!(patches.len(), 2); // 1 file + 1 memory

        let file_patch = patches
            .iter()
            .find(|p| p.patch_type == PatchType::File)
            .expect("file patch present");
        let mem_patch = patches
            .iter()
            .find(|p| p.patch_type == PatchType::Memory)
            .expect("memory patch present");

        for p in [&file_patch, &mem_patch] {
            assert_eq!(p.feature, FeatureType::AntiTelemetry);
            assert!(!p.use_regex);
            assert!(p.regex_replace_values.is_none());
            assert!(p.patch_byte.is_none());
            assert!(p.patch_offset.is_none());
            // 等长替换铁律：27 -> 27
            let search: &[u8] = p.search_pattern.as_ref();
            let replace: &[u8] = p
                .replace_pattern
                .as_ref()
                .map(|c| c.as_ref())
                .expect("replace_pattern present for antitelemetry");
            assert_eq!(search.len(), 27);
            assert_eq!(replace.len(), 27);
            assert_eq!(search, &b"/api/event_logging/v2/batch"[..]);
            assert_eq!(replace, &b"/api/event_logging/v2/xxxxx"[..]);
        }
    }

    #[test]
    fn test_antitelemetry_via_get_feature_patches() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 195,
        };
        let patches = get_feature_patches(FeatureType::AntiTelemetry, &version);
        assert_eq!(patches.len(), 2);
        assert!(patches.iter().all(|p| !p.use_regex));
    }

    #[test]
    fn test_antitelemetry_patches_equal_length() {
        // 等长替换铁律：search 与 replace 必须等长，否则会破坏二进制偏移
        let patches = get_antitelemetry_patches();
        for p in &patches {
            let s = p.search_pattern.as_ref();
            let r = p.replace_pattern.as_ref().unwrap();
            assert_eq!(
                s.len(),
                r.len(),
                "antitelemetry patch must be equal length"
            );
        }
    }
}
