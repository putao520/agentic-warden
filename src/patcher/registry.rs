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
        FeatureType::AntiSpy => get_antispy_patches(),
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

/// 生成 AntiSpy patch 模式
///
/// 让 CC 本地识别全失明：
/// - `KIt()` → 返回 `"UTC"`：时区永远返回 UTC，真实时区不泄露，cnTZ 永远 false
/// - `Hsp()` → 返回 `null`：中转站识别返回 null，known/labKw/cnTZ/host 全 null
///
/// 函数级 patch（非字符串刮花），等长字面量替换，跨版本稳定。
/// 不碰 `$Sn()`（保留 firstParty 专属功能）。
///
/// 返回文件补丁与内存补丁各两份（KIt + Hsp，共 4 个 patch）。
pub fn get_antispy_patches() -> Vec<UnifiedPatchPattern> {
    // patch 1: KIt() → "UTC"（时区失明）
    // search: `Intl.DateTimeFormat().resolvedOptions().timeZone` (48 字节)
    // replace: `"UTC"/*` + 39 个 `.` + `*/` (48 字节，注释填充，JS 合法)
    let tz_search = b"Intl.DateTimeFormat().resolvedOptions().timeZone";
    let mut tz_replace_full = Vec::with_capacity(48);
    tz_replace_full.extend_from_slice(b"\"UTC\"/*");
    tz_replace_full.extend(std::iter::repeat_n(b'.', 39));
    tz_replace_full.extend_from_slice(b"*/");
    assert_eq!(
        tz_search.len(),
        tz_replace_full.len(),
        "timezone patch must be equal length"
    );

    // patch 2: Hsp() → null（中转站失明）
    // search: `function Hsp(){if($Sn())return null;let e=Asp()` (47 字节)
    // replace: `function Hsp(){return null;         let e=Asp()` (47 字节，空格填充)
    let hsp_search = b"function Hsp(){if($Sn())return null;let e=Asp()";
    let hsp_replace = b"function Hsp(){return null;         let e=Asp()";
    assert_eq!(
        hsp_search.len(),
        hsp_replace.len(),
        "hsp patch must be equal length"
    );

    vec![
        // patch 1: KIt() → UTC（File + Memory 两个 patch）
        UnifiedPatchPattern {
            feature: FeatureType::AntiSpy,
            patch_type: PatchType::File,
            search_pattern: Cow::Owned(tz_search.to_vec()),
            replace_pattern: Some(Cow::Owned(tz_replace_full.clone())),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("AntiSpy file patch: KIt() timezone -> UTC"),
            use_regex: false,
            regex_replace_values: None,
        },
        UnifiedPatchPattern {
            feature: FeatureType::AntiSpy,
            patch_type: PatchType::Memory,
            search_pattern: Cow::Owned(tz_search.to_vec()),
            replace_pattern: Some(Cow::Owned(tz_replace_full)),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("AntiSpy memory patch: KIt() timezone -> UTC"),
            use_regex: false,
            regex_replace_values: None,
        },
        // patch 2: Hsp() → null（File + Memory 两个 patch）
        UnifiedPatchPattern {
            feature: FeatureType::AntiSpy,
            patch_type: PatchType::File,
            search_pattern: Cow::Owned(hsp_search.to_vec()),
            replace_pattern: Some(Cow::Owned(hsp_replace.to_vec())),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("AntiSpy file patch: Hsp() relay detection -> null"),
            use_regex: false,
            regex_replace_values: None,
        },
        UnifiedPatchPattern {
            feature: FeatureType::AntiSpy,
            patch_type: PatchType::Memory,
            search_pattern: Cow::Owned(hsp_search.to_vec()),
            replace_pattern: Some(Cow::Owned(hsp_replace.to_vec())),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("AntiSpy memory patch: Hsp() relay detection -> null"),
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

    #[test]
    fn test_antispy_patches_structure() {
        let patches = get_antispy_patches();
        assert_eq!(patches.len(), 4); // KIt(file+memory) + Hsp(file+memory)

        let file_count = patches
            .iter()
            .filter(|p| p.patch_type == PatchType::File)
            .count();
        let mem_count = patches
            .iter()
            .filter(|p| p.patch_type == PatchType::Memory)
            .count();
        assert_eq!(file_count, 2);
        assert_eq!(mem_count, 2);

        for p in &patches {
            assert_eq!(p.feature, FeatureType::AntiSpy);
            assert!(!p.use_regex);
            assert!(p.regex_replace_values.is_none());
            assert!(p.patch_byte.is_none());
            assert!(p.patch_offset.is_none());
        }
    }

    #[test]
    fn test_antispy_patches_equal_length() {
        // 等长替换铁律：search 与 replace 必须等长
        let patches = get_antispy_patches();
        for p in &patches {
            let s = p.search_pattern.as_ref();
            let r = p.replace_pattern.as_ref().unwrap();
            assert_eq!(s.len(), r.len(), "antispy patch must be equal length");
        }
    }

    #[test]
    fn test_antispy_timezone_patch_48_bytes() {
        // KIt() patch: 48 字节 search → 48 字节 replace
        let patches = get_antispy_patches();
        let tz_patch = patches
            .iter()
            .find(|p| p.description.contains("KIt()"))
            .expect("KIt timezone patch present");
        let search: &[u8] = tz_patch.search_pattern.as_ref();
        let replace: &[u8] = tz_patch.replace_pattern.as_ref().unwrap().as_ref();
        assert_eq!(search.len(), 48);
        assert_eq!(replace.len(), 48);
        assert_eq!(search, &b"Intl.DateTimeFormat().resolvedOptions().timeZone"[..]);
        // replace 以 "UTC"/* 开头，以 */ 结尾，中间 39 个 .
        assert!(replace.starts_with(b"\"UTC\"/*"));
        assert!(replace.ends_with(b"*/"));
        let middle = &replace[7..replace.len() - 2];
        assert_eq!(middle.len(), 39);
        assert!(middle.iter().all(|&b| b == b'.'));
    }

    #[test]
    fn test_antispy_hsp_patch_47_bytes() {
        // Hsp() patch: 47 字节 search → 47 字节 replace
        let patches = get_antispy_patches();
        let hsp_patch = patches
            .iter()
            .find(|p| p.description.contains("Hsp()"))
            .expect("Hsp relay patch present");
        let search: &[u8] = hsp_patch.search_pattern.as_ref();
        let replace: &[u8] = hsp_patch.replace_pattern.as_ref().unwrap().as_ref();
        assert_eq!(search.len(), 47);
        assert_eq!(replace.len(), 47);
        assert_eq!(
            search,
            &b"function Hsp(){if($Sn())return null;let e=Asp()"[..]
        );
        assert_eq!(
            replace,
            &b"function Hsp(){return null;         let e=Asp()"[..]
        );
    }

    #[test]
    fn test_antispy_via_get_feature_patches() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 195,
        };
        let patches = get_feature_patches(FeatureType::AntiSpy, &version);
        assert_eq!(patches.len(), 4);
        assert!(patches.iter().all(|p| !p.use_regex));
        assert!(patches.iter().all(|p| p.feature == FeatureType::AntiSpy));
    }
}
