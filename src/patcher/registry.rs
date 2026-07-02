//! Patch registry - generates UnifiedPatchPattern for MaxContextTokens patch

use crate::patcher::types::{FeatureType, PatchType, UnifiedPatchPattern};
use crate::patcher::versions::{
    validate_max_context_tokens, ClaudeVersion, MAX_CONTEXT_TOKENS_SEARCH_REGEX,
};
use std::borrow::Cow;

/// Get patches for a feature based on version signature
///
/// 目前支持 MaxContextTokens / AntiTelemetry / AntiSpy / AntiPromptBias 四种功能；
/// 保留 version 参数以兼容调用方签名（regex/字面量均不依赖版本签名）。
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
        FeatureType::AntiPromptBias => get_antipromptbias_patches(),
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
/// 让 CC 本地识别全失明，两个 patch 各 File + Memory 共 4 个：
///
/// - **patch A（逃生口短路，regex 字面量模式）**：把
///   `if(<OBJ>._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0` 改成
///   `if(1)` + 50 空格（共 55 字节，`if(1)` 短路，50 空格填充等长）。
///   `<OBJ>` 跨版本变化（195-197 用 `Oe`，198 用 `Pe`），故用 regex 通配
///   `[a-zA-Z_$][a-zA-Z0-9_$]*` 匹配对象名。`_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL`
///   是稳定字面量锚点。逃生口短路后，所有 firstParty 识别恒真，中转站/云
///   识别全 null（不再泄露 host/known/cnTZ 等信息）。
///
/// - **patch B（时区失明，字面量模式）**：`Intl.DateTimeFormat().resolvedOptions().timeZone`
///   （48 字节）→ `"UTC"/*` + 39 个 `.` + `*/`（48 字节注释填充）。时区永远
///   返回 UTC，真实时区不泄露，`cnTZ` 永远 false。
///
/// 跨版本验证（195-198）：逃生口 55 字节各 1 处，时区 48 字节各 2 处。
/// 等长替换铁律：A=55→55，B=48→48。
pub fn get_antispy_patches() -> Vec<UnifiedPatchPattern> {
    // patch A: 逃生口短路（regex 字面量模式）
    // search (regex): `if(<OBJ>._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0`
    //   <OBJ> 跨版本变化（Oe/Pe/...），用 [a-zA-Z_$][a-zA-Z0-9_$]* 通配
    // replace (字面量, 55B): `if(1)` + 50 空格
    let escape_search: Cow<'static, [u8]> = Cow::Borrowed(
        br"if\([a-zA-Z_$][a-zA-Z0-9_$]*\._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL\)return!0"
            .as_ref(),
    );
    // 运行时构造 replace：if(1) + 50 空格 = 55 字节
    let mut escape_replace_vec = Vec::with_capacity(55);
    escape_replace_vec.extend_from_slice(b"if(1)");
    escape_replace_vec.extend(std::iter::repeat_n(b' ', 50));
    debug_assert_eq!(escape_replace_vec.len(), 55, "escape patch replace must be 55 bytes");
    let escape_replace: Cow<'static, [u8]> = Cow::Owned(escape_replace_vec);

    // patch B: KIt() → UTC（时区失明，字面量模式）
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

    vec![
        // patch A: 逃生口短路（File + Memory，regex 字面量模式）
        UnifiedPatchPattern {
            feature: FeatureType::AntiSpy,
            patch_type: PatchType::File,
            search_pattern: escape_search.clone(),
            replace_pattern: Some(escape_replace.clone()),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiSpy file patch: escape hatch short-circuit (firstParty assume -> true)",
            ),
            use_regex: true,
            regex_replace_values: None,
        },
        UnifiedPatchPattern {
            feature: FeatureType::AntiSpy,
            patch_type: PatchType::Memory,
            search_pattern: escape_search,
            replace_pattern: Some(escape_replace),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiSpy memory patch: escape hatch short-circuit (firstParty assume -> true)",
            ),
            use_regex: true,
            regex_replace_values: None,
        },
        // patch B: KIt() → UTC（File + Memory，字面量模式）
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
    ]
}

/// 生成 AntiPromptBias patch 模式
///
/// 消除 CC 给第三方用户注入的 Provider context 提示词偏见：把
/// `if(<FN>())n.push("**Provider context:** This session is not using")`
/// 改成 `if(0   )n.push("**Provider context:** This session is not using")`
/// 让条件永远 false → Provider context prompt 不注入，模型不感知 provider
/// 差异，行为更一致。只跳过这一条 prompt，不影响其他 firstParty 门控
/// （OAuth/能力/模型选择等照常）。
///
/// **regex 字面量替换模式**（`use_regex=true` + `replace_pattern=Some`）：
/// `<FN>` 跨版本变化（195 `g7`、196 `F7`、197 `j7`、198 `dX`，都是 2 字符），
/// 故用 `if\([a-zA-Z_$][a-zA-Z0-9_$]*\(\)\)` regex 通配函数名。regex 匹配的
/// 文本固定 63 字节（`if(XX())` 7 字节 + 后续 56 字节 prompt），replace 是
/// 固定字面量 63 字节（`if(0   )` 7B + 后续 56B 相同 prompt），等长覆盖。
///
/// 跨版本稳定（prompt 字面量 + regex 通配函数名）。返回文件补丁与内存补丁各一份。
pub fn get_antipromptbias_patches() -> Vec<UnifiedPatchPattern> {
    // search (regex): `if(<FN>())n.push("**Provider context:** This session is not using`
    //   <FN> 跨版本变化（g7/F7/j7/dX，2 字符），regex 通配
    //   regex 匹配文本固定 63 字节：if(XX())=7B + prompt=56B
    // replace (字面量, 63B): `if(0   )` (7B, 3 空格) + prompt (56B, 与 search 后缀一致)
    let search: Cow<'static, [u8]> = Cow::Borrowed(
        br#"if\([a-zA-Z_$][a-zA-Z0-9_$]*\(\)\)n\.push\("\*\*Provider context:\*\* This session is not using"#
            .as_ref(),
    );
    let replace: Cow<'static, [u8]> =
        Cow::Borrowed(br#"if(0   )n.push("**Provider context:** This session is not using"#);
    // 等长校验：regex 匹配文本 63B（XX=2 字符时），replace 63B
    // 注：replace 是字面量，长度固定 63；regex 匹配长度随 <FN> 字符数变化，
    // 但实测 195-198 的 <FN> 都是 2 字符（g7/F7/j7/dX），匹配文本恒 63B
    assert_eq!(
        replace.len(),
        63,
        "antipromptbias replace must be 63 bytes"
    );

    vec![
        UnifiedPatchPattern {
            feature: FeatureType::AntiPromptBias,
            patch_type: PatchType::File,
            search_pattern: search.clone(),
            replace_pattern: Some(replace.clone()),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiPromptBias file patch: skip Provider context prompt for 3P",
            ),
            use_regex: true,
            regex_replace_values: None,
        },
        UnifiedPatchPattern {
            feature: FeatureType::AntiPromptBias,
            patch_type: PatchType::Memory,
            search_pattern: search,
            replace_pattern: Some(replace),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiPromptBias memory patch: skip Provider context prompt for 3P",
            ),
            use_regex: true,
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
        assert_eq!(patches.len(), 4); // escape(file+memory) + KIt(file+memory)

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
            assert!(p.regex_replace_values.is_none());
            assert!(p.patch_byte.is_none());
            assert!(p.patch_offset.is_none());
            assert!(p.replace_pattern.is_some());
        }
    }

    #[test]
    fn test_antispy_patches_equal_length() {
        // 等长替换铁律：每个 patch 的 regex 匹配文本长度必须 == replace_pattern 长度
        // - 逃生口 patch：regex 匹配 55B（Oe/Pe 2字符对象名），replace 55B
        // - 时区 patch：字面量 48B → 48B
        let patches = get_antispy_patches();
        for p in &patches {
            let replace = p.replace_pattern.as_ref().unwrap();
            if p.use_regex {
                // regex 模式：编译 regex 并对样本验证匹配长度 == replace 长度
                let regex_str = std::str::from_utf8(p.search_pattern.as_ref()).unwrap();
                let re = regex::bytes::Regex::new(regex_str).unwrap();
                // 逃生口样本：195-197 用 Oe，198 用 Pe
                for sample in [
                    b"if(Oe._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0".as_ref(),
                    b"if(Pe._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0".as_ref(),
                ] {
                    let m = re.find(sample).unwrap_or_else(|| {
                        panic!("escape regex must match sample: {}", String::from_utf8_lossy(sample))
                    });
                    assert_eq!(
                        m.end() - m.start(),
                        replace.len(),
                        "escape regex match length must == replace length (55)"
                    );
                }
                assert_eq!(replace.len(), 55, "escape patch replace must be 55 bytes");
            } else {
                // 字面量模式：search 与 replace 等长
                let s = p.search_pattern.as_ref();
                assert_eq!(s.len(), replace.len(), "literal antispy patch must be equal length");
            }
        }
    }

    #[test]
    fn test_antispy_timezone_patch_48_bytes() {
        // KIt() patch: 48 字节 search → 48 字节 replace（字面量模式）
        let patches = get_antispy_patches();
        let tz_patch = patches
            .iter()
            .find(|p| p.description.contains("KIt()"))
            .expect("KIt timezone patch present");
        assert!(!tz_patch.use_regex, "timezone patch is literal mode");
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
    fn test_antispy_escape_hatch_patch_55_bytes() {
        // 逃生口 patch: regex 匹配 55B → replace 55B（regex 字面量模式）
        let patches = get_antispy_patches();
        let escape_patch = patches
            .iter()
            .find(|p| p.description.contains("escape hatch"))
            .expect("escape hatch patch present");
        assert!(escape_patch.use_regex, "escape patch is regex mode");
        let replace: &[u8] = escape_patch.replace_pattern.as_ref().unwrap().as_ref();
        assert_eq!(replace.len(), 55, "escape replace must be 55 bytes");
        // replace = if(1) (5B) + 50 空格
        assert_eq!(&replace[..5], b"if(1)");
        assert_eq!(&replace[5..], &[b' '; 50][..], "escape replace must be if(1) + 50 spaces");

        // regex 匹配 195-198 各版本样本，匹配长度恒 55
        let regex_str = std::str::from_utf8(escape_patch.search_pattern.as_ref()).unwrap();
        let re = regex::bytes::Regex::new(regex_str).unwrap();
        let samples: [(&[u8], &str); 2] = [
            (b"if(Oe._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0".as_ref(), "195-197 Oe"),
            (b"if(Pe._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0".as_ref(), "198 Pe"),
            // 3 字符对象名也应匹配（regex 通配），但长度会变 56，replace 不适用
            // 这里只验证 2 字符对象名（实测 195-198 都是 2 字符）
        ];
        for (sample, label) in samples {
            let m = re.find(sample).unwrap_or_else(|| {
                panic!("escape regex must match {} sample", label)
            });
            assert_eq!(m.end() - m.start(), 55, "escape regex match length for {} must be 55", label);
        }
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
        assert!(patches.iter().all(|p| p.feature == FeatureType::AntiSpy));
        // 2 个 regex（逃生口）+ 2 个字面量（时区）
        let regex_count = patches.iter().filter(|p| p.use_regex).count();
        let literal_count = patches.iter().filter(|p| !p.use_regex).count();
        assert_eq!(regex_count, 2);
        assert_eq!(literal_count, 2);
    }

    #[test]
    fn test_antipromptbias_patches_structure() {
        let patches = get_antipromptbias_patches();
        assert_eq!(patches.len(), 2); // 1 file + 1 memory

        let file_count = patches
            .iter()
            .filter(|p| p.patch_type == PatchType::File)
            .count();
        let mem_count = patches
            .iter()
            .filter(|p| p.patch_type == PatchType::Memory)
            .count();
        assert_eq!(file_count, 1);
        assert_eq!(mem_count, 1);

        for p in &patches {
            assert_eq!(p.feature, FeatureType::AntiPromptBias);
            assert!(p.use_regex, "antipromptbias is regex mode");
            assert!(p.regex_replace_values.is_none());
            assert!(p.patch_byte.is_none());
            assert!(p.patch_offset.is_none());
            assert!(p.replace_pattern.is_some());
        }
    }

    #[test]
    fn test_antipromptbias_patches_equal_length() {
        // 等长替换铁律：regex 匹配文本长度（63B，XX=2字符）== replace_pattern 长度（63B）
        let patches = get_antipromptbias_patches();
        let p = &patches[0];
        let replace = p.replace_pattern.as_ref().unwrap();
        assert_eq!(replace.len(), 63, "replace must be 63 bytes");

        let regex_str = std::str::from_utf8(p.search_pattern.as_ref()).unwrap();
        let re = regex::bytes::Regex::new(regex_str).unwrap();
        // 195-198 各版本样本，函数名 2 字符，匹配文本恒 63B
        let samples: [(&[u8], &str); 4] = [
            (br#"if(g7())n.push("**Provider context:** This session is not using"#, "195 g7"),
            (br#"if(F7())n.push("**Provider context:** This session is not using"#, "196 F7"),
            (br#"if(j7())n.push("**Provider context:** This session is not using"#, "197 j7"),
            (br#"if(dX())n.push("**Provider context:** This session is not using"#, "198 dX"),
        ];
        for (sample, label) in samples {
            let m = re.find(sample).unwrap_or_else(|| {
                panic!("antipromptbias regex must match {} sample", label)
            });
            assert_eq!(
                m.end() - m.start(),
                63,
                "antipromptbias regex match length for {} must be 63",
                label
            );
        }
    }

    #[test]
    fn test_antipromptbias_patch_63_bytes() {
        // AntiPromptBias patch: regex 匹配 63B → replace 63B
        let patches = get_antipromptbias_patches();
        for p in &patches {
            assert!(p.use_regex);
            let replace: &[u8] = p.replace_pattern.as_ref().unwrap().as_ref();
            assert_eq!(replace.len(), 63);
            // replace 以 if(0   ) (8B) 开头，后接 prompt 字面量 (55B)
            assert_eq!(&replace[..8], b"if(0   )");
            assert_eq!(
                &replace[8..],
                &br#"n.push("**Provider context:** This session is not using"#[..]
            );
        }
    }

    #[test]
    fn test_antipromptbias_via_get_feature_patches() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 195,
        };
        let patches = get_feature_patches(FeatureType::AntiPromptBias, &version);
        assert_eq!(patches.len(), 2);
        assert!(patches.iter().all(|p| p.use_regex));
        assert!(patches.iter().all(|p| p.feature == FeatureType::AntiPromptBias));
    }
}
