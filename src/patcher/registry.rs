//! Patch registry - generates UnifiedPatchPattern for MaxContextTokens patch

use crate::patcher::types::{FeatureType, PatchType, UnifiedPatchPattern};
use crate::patcher::versions::{
    validate_max_context_tokens, ClaudeVersion, MAX_CONTEXT_TOKENS_SEARCH_REGEX,
};
use std::borrow::Cow;

/// Get patches for a feature based on version signature
///
/// 目前仅 MaxContextTokens 一种功能；保留 feature 参数以兼容调用方签名。
pub fn get_feature_patches(
    feature: FeatureType,
    version: &ClaudeVersion,
) -> Vec<UnifiedPatchPattern> {
    match feature {
        FeatureType::MaxContextTokens => {
            get_max_context_tokens_patches(version, 500000, 500000)
        }
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
}
