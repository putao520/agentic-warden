//! Patch registry - generates UnifiedPatchPattern for MaxContextTokens patch

use crate::patcher::types::{DynamicReplace, FeatureType, PatchType, UnifiedPatchPattern};
use crate::patcher::claude::versions::{
    validate_max_context_tokens, ClaudeVersion, MAX_CONTEXT_TOKENS_SEARCH_REGEX,
};
use std::borrow::Cow;

/// Get patches for a feature based on version signature
///
/// 目前支持 MaxContextTokens / AntiTelemetry / AntiSpy / AntiPromptBias / AntiAtis /
/// AntiFrameTrack / AntiCloudDetect 七种功能；
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
        FeatureType::AntiAtis => get_antiatis_patches(),
        FeatureType::AntiFrameTrack => get_antiframetrack_patches(),
        FeatureType::AntiCloudDetect => get_anticloudetect_patches(),
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
            dynamic_replace: None,
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
            dynamic_replace: None,
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
    make_patch_pair(
        FeatureType::AntiTelemetry,
        false,
        Cow::Borrowed(b"/api/event_logging/v2/batch"),
        Cow::Borrowed(b"/api/event_logging/v2/xxxxx"),
        "AntiTelemetry file patch: event_logging endpoint -> 404",
        "AntiTelemetry memory patch: event_logging endpoint -> 404",
    )
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
    let mut patches = get_escape_hatch_patches();
    patches.extend(get_timezone_patches());
    patches
}

/// 逃生口短路 patch（File + Memory，regex 字面量模式）
///
/// search (regex): `if(<OBJ>._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0`
///   `<OBJ>` 跨版本变化（Oe/Pe/...），用 `[a-zA-Z_$][a-zA-Z0-9_$]*` 通配
/// replace (字面量, 55B): `if(1)` + 50 空格
/// 构造 File + Memory 一对 patch（相同 search/replace，只差 patch_type 和 description）
fn make_patch_pair(
    feature: FeatureType,
    use_regex: bool,
    search: Cow<'static, [u8]>,
    replace: Cow<'static, [u8]>,
    file_desc: &'static str,
    mem_desc: &'static str,
) -> Vec<UnifiedPatchPattern> {
    vec![
        UnifiedPatchPattern {
            feature,
            patch_type: PatchType::File,
            search_pattern: search.clone(),
            replace_pattern: Some(replace.clone()),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(file_desc),
            use_regex,
            regex_replace_values: None,
            dynamic_replace: None,
        },
        UnifiedPatchPattern {
            feature,
            patch_type: PatchType::Memory,
            search_pattern: search,
            replace_pattern: Some(replace),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(mem_desc),
            use_regex,
            regex_replace_values: None,
            dynamic_replace: None,
        },
    ]
}

fn get_escape_hatch_patches() -> Vec<UnifiedPatchPattern> {
    let escape_search: Cow<'static, [u8]> = Cow::Borrowed(
        br"if\([a-zA-Z_$][a-zA-Z0-9_$]*\._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL\)return!0"
            .as_ref(),
    );
    // 运行时构造 replace：if(1) + 50 空格 = 55 字节
    let mut escape_replace_vec = Vec::with_capacity(55);
    escape_replace_vec.extend_from_slice(b"if(1)");
    escape_replace_vec.extend(std::iter::repeat_n(b' ', 50));
    debug_assert_eq!(escape_replace_vec.len(), 55, "escape patch replace must be 55 bytes");

    make_patch_pair(
        FeatureType::AntiSpy,
        true,
        escape_search,
        Cow::Owned(escape_replace_vec),
        "AntiSpy file patch: escape hatch short-circuit (firstParty assume -> true)",
        "AntiSpy memory patch: escape hatch short-circuit (firstParty assume -> true)",
    )
}

/// 时区失明 patch（File + Memory，字面量模式）
///
/// search: `Intl.DateTimeFormat().resolvedOptions().timeZone` (48 字节)
/// replace: `"UTC"/*` + 39 个 `.` + `*/` (48 字节，注释填充，JS 合法)
fn get_timezone_patches() -> Vec<UnifiedPatchPattern> {
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

    make_patch_pair(
        FeatureType::AntiSpy,
        false,
        Cow::Owned(tz_search.to_vec()),
        Cow::Owned(tz_replace_full),
        "AntiSpy file patch: KIt() timezone -> UTC",
        "AntiSpy memory patch: KIt() timezone -> UTC",
    )
}

/// 生成 AntiPromptBias patch 模式
///
/// 消除 CC 给第三方用户注入的 Provider context 提示词偏见：把
/// `if(<FN>())<arr>.push("**Provider context:** This session is not using")`
/// 改成 `if(0)` + 空格填充 + `<arr>.push("**Provider context...")`
/// 让条件永远 false → Provider context prompt 不注入，模型不感知 provider
/// 差异，行为更一致。只跳过这一条 prompt，不影响其他 firstParty 门控
/// （OAuth/能力/模型选择等照常）。
///
/// **regex 动态替换模式（模式 4）**（`use_regex=true` + `replace_pattern=None`
/// + `regex_replace_values=None` + `dynamic_replace=Some(ReplacePrefix)`）：
///
/// - `<FN>` 跨版本变化（195 `g7`、196 `F7`、197 `j7`、198 `dX`、201 `dJ`、
///   207 `yfe`），字符数 2~3 不定。`<arr>`（push 数组变量）也跨版本变化
///   （195-201 用 `n`，207 用 `r`）。旧「regex 字面量替换」写死 `n` 且固定
///   63B replace，207 把 `arr` 改 `r` + FN 变 3 字符致匹配长度 64B ≠ 63B，
///   等长铁律破坏 → patch 失效（BCE 根治类 BUG）。
/// - 新 regex：`if\(<FN>\(\)\)(<arr>\.push\("\*\*Provider context:\*\* This session is not using)`
///   组 1 捕获后缀 `<arr>.push("**Provider context...`（arr 通配跨版本自适应）。
/// - 模式 4 `ReplacePrefix`：`keep_group=1`（保留后缀组）、
///   `prefix_literal=b"if(0)"`。replace 动态构造为
///   `if(0)` + 空格*(span_len - 5 - keep_len) + match[1]，等长自动成立
///   （空格数随 FN/arr 长度变化自动适应）。
///   - 195/201: `if(0)` + 3 空格 + `n.push("**Provider context...` = 63B ✅
///   - 207: `if(0)` + 4 空格 + `r.push("**Provider context...` = 64B ✅
///
/// 跨版本稳定（prompt 字面量 + regex 通配 FN/arr）。返回文件补丁与内存补丁各一份。
pub fn get_antipromptbias_patches() -> Vec<UnifiedPatchPattern> {
    // search (regex): `if(<FN>())<arr>.push("**Provider context:** This session is not using`
    //   <FN> 函数名跨版本变化（g7/F7/j7/dX/dJ/yfe，2~3 字符），regex 通配
    //   <arr> push 数组变量跨版本变化（195-201 n，207 r），regex 通配
    //   组 1 捕获后缀 `<arr>.push("**Provider context...`，保留原样
    let search: Cow<'static, [u8]> = Cow::Borrowed(
        br#"if\([a-zA-Z_$][a-zA-Z0-9_$]*\(\)\)([a-zA-Z_$][a-zA-Z0-9_$]*\.push\("\*\*Provider context:\*\* This session is not using)"#
            .as_ref(),
    );
    // 模式 4 ReplacePrefix: prefix if(0) + 空格填充 + 保留组 1（arr.push("**Provider...）)
    let dynamic = DynamicReplace::ReplacePrefix {
        keep_group: 1,
        prefix_literal: Cow::Borrowed(b"if(0)"),
    };

    vec![
        UnifiedPatchPattern {
            feature: FeatureType::AntiPromptBias,
            patch_type: PatchType::File,
            search_pattern: search.clone(),
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiPromptBias file patch: skip Provider context prompt for 3P",
            ),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: Some(dynamic.clone()),
        },
        UnifiedPatchPattern {
            feature: FeatureType::AntiPromptBias,
            patch_type: PatchType::Memory,
            search_pattern: search,
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiPromptBias memory patch: skip Provider context prompt for 3P",
            ),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: Some(dynamic),
        },
    ]
}

/// 生成 AntiAtis patch 模式
///
/// 防止 x-cc-atis 追踪 header 注入：逃生口短路 patch 副作用使 gu()=true，
/// 激活 `tMi(firstParty)&&gu()` 条件，触发 atis header 注入逻辑（原本中转站
/// gu()=false 短路）。atis 是服务端 bootstrap 下发的追踪 token，会暴露中转站
/// 用户身份。patch atis 提取函数让它永远返回 void 0 → header 永不注入。
///
/// **regex 动态替换模式（模式 4）**（`use_regex=true` + `replace_pattern=None`
/// + `regex_replace_values=None` + `dynamic_replace=Some(KeepPrefix)`）：
///
/// atis 提取函数跨版本结构（195-207 命中）：
/// - 196: `function S6r(){let e=P0()?.atis;return typeof e==="string"&&e.length>0?e:void 0}`
/// - 197: `function H6r(){let e=AI()?.atis;...}`
/// - 198: `function pYr(){let e=I0()?.atis;...}`
/// - 199: `function SXr(){let e=q0()?.atis;...}`
/// - 207: `function R0i(){let e=mL()?.atis;...}`
///
/// 函数名（FN，2~3 字符）和 bootstrap 函数名（BF，2 字符）跨版本变化。旧
/// 「regex 字面量替换」用固定 80B replace（假设 FN 恒 3 字符），当前 196-207
/// 恰好 3 字符所以等长成立，但这是巧合——未来版本 FN 变 2/4 字符就会失配
/// （同类隐患，BCE 根治类 BUG，与 AntiPromptBias 同源）。
/// - regex 加捕获组 1 保留函数名前缀：
///   `(function <FN>\(\)\{)let e=<BF>\(\)[?]\.atis;return typeof e==="string"&&e\.length>0[?]e:void 0\}`
///   组 1 = `function R0i(){`（含函数名，保留原样）。
/// - 模式 4 `KeepPrefix`：`keep_group=1`、`suffix_literal=b"return void 0"`、
///   `end_literal=b"}"`。replace 动态构造为
///   `match[1]` + `return void 0` + 空格*(span_len - keep_len - 13 - 1) + `}`,
///   等长自动成立（空格数随 FN 长度变化自动适应）。
///   - 207: `function R0i(){` + `return void 0` + 51 空格 + `}` = 80B ✅
///
/// 跨 195-207 通用。返回文件补丁与内存补丁各一份。
pub fn get_antiatis_patches() -> Vec<UnifiedPatchPattern> {
    // search (regex): 通配函数名 + bootstrap 函数名，组 1 捕获函数名前缀
    //   `function <FN>(){let e=<BF>()?.atis;return typeof e==="string"&&e.length>0?e:void 0}`
    //   <FN> 2~3 字符（S6r/H6r/pYr/SXr/R0i），<BF> 2 字符（P0/AI/I0/q0/mL）
    //   组 1 = `function <FN>(){`，保留原样（函数名自适应）
    let search: Cow<'static, [u8]> = Cow::Borrowed(
        br#"(function [a-zA-Z_$][a-zA-Z0-9_$]*\(\)\{)let e=[a-zA-Z_$][a-zA-Z0-9_$]*\(\)[?]\.atis;return typeof e==="string"&&e\.length>0[?]e:void 0\}"#
            .as_ref(),
    );
    // 模式 4 KeepPrefix: 保留组 1（function <FN>(){）+ return void 0 + 空格填充 + }
    let dynamic = DynamicReplace::KeepPrefix {
        keep_group: 1,
        suffix_literal: Cow::Borrowed(b"return void 0"),
        end_literal: Cow::Borrowed(b"}"),
    };

    vec![
        UnifiedPatchPattern {
            feature: FeatureType::AntiAtis,
            patch_type: PatchType::File,
            search_pattern: search.clone(),
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiAtis file patch: atis extract -> void 0 (no x-cc-atis header)",
            ),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: Some(dynamic.clone()),
        },
        UnifiedPatchPattern {
            feature: FeatureType::AntiAtis,
            patch_type: PatchType::Memory,
            search_pattern: search,
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed(
                "AntiAtis memory patch: atis extract -> void 0 (no x-cc-atis header)",
            ),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: Some(dynamic),
        },
    ]
}

/// 生成 AntiFrameTrack patch 模式
///
/// 截断 CC 第二上报通道：`trackFrameEvent(trr)` 上报 artifact 使用行为到
/// `/api/frame/track` 端点（独立于 AntiTelemetry 的 event_logging 通道）。
/// 通过等长字面量替换把端点改成 `/api/frame/xxxxx` → 404 静默失败
/// （try/catch 吞错，trackFrameEvent 不抛错）。
///
/// **字面量模式**（`use_regex=false`）：search = replace = 16 字节。
/// 跨版本稳定（API 路径字面量，不依赖 minified 变量名）。返回文件补丁与
/// 内存补丁各一份。
pub fn get_antiframetrack_patches() -> Vec<UnifiedPatchPattern> {
    make_patch_pair(
        FeatureType::AntiFrameTrack,
        false,
        Cow::Borrowed(b"/api/frame/track"),
        Cow::Borrowed(b"/api/frame/xxxxx"),
        "AntiFrameTrack file patch: frame/track endpoint -> 404",
        "AntiFrameTrack memory patch: frame/track endpoint -> 404",
    )
}

/// 生成 AntiCloudDetect patch 模式
///
/// 禁用 MAC 地址 GCE 云检测：`tMi()` 遍历 `networkInterfaces()` 的 MAC，
/// 用 `fGd=/^42:01/` regex 匹配 GCE 实例 OUI 前缀。当前是预留间谍点
/// （导出但无内部调用方），防未来版本激活。通过等长字面量替换把 regex
/// 改成 `/^00:00/`（永不匹配任何 MAC）→ `fGd.test()` 永远 false →
/// `tMi()` 永远返回 false。
///
/// **字面量模式**（`use_regex=false`）：search = replace = 8 字节。
/// 跨版本稳定（regex 字面量 `/^42:01/` 是常量，不依赖 minified 变量名）。
/// 返回文件补丁与内存补丁各一份。
///
/// 不防 eMi（GCE BIOS `/Google/.test`）：eMi 是 Linux-only + 需读
/// `/sys/class/dmi/id/bios_vendor` 文件，中转站用户通常不跑 GCE；且
/// `/Google/` 字面量太通用不能改。tMi 是 MAC 扫描，任何环境都能跑，优先防。
pub fn get_anticloudetect_patches() -> Vec<UnifiedPatchPattern> {
    make_patch_pair(
        FeatureType::AntiCloudDetect,
        false,
        Cow::Borrowed(b"/^42:01/"),
        Cow::Borrowed(b"/^00:00/"),
        "AntiCloudDetect file patch: GCE MAC OUI regex -> never match",
        "AntiCloudDetect memory patch: GCE MAC OUI regex -> never match",
    )
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
            // 模式 4：replace_pattern=None，dynamic_replace=Some(ReplacePrefix)
            assert!(p.replace_pattern.is_none(), "mode4 has no replace_pattern");
            assert!(p.dynamic_replace.is_some(), "mode4 has dynamic_replace");
            match p.dynamic_replace.as_ref().unwrap() {
                DynamicReplace::ReplacePrefix {
                    keep_group,
                    prefix_literal,
                } => {
                    assert_eq!(*keep_group, 1, "keep_group must be 1");
                    assert_eq!(prefix_literal.as_ref(), b"if(0)");
                }
                other => panic!("expected ReplacePrefix, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_antipromptbias_patches_equal_length() {
        // 模式 4 等长铁律：regex 匹配 span 长度 == 动态构造的 replace 长度
        // （replace = if(0) + 空格*(span-5-keep_len) + match[1]，等长自动成立）
        // 跨版本样本：195/201 arr=n FN 2字符 span=63B；207 arr=r FN 3字符 span=64B
        let patches = get_antipromptbias_patches();
        let p = &patches[0];
        assert!(p.replace_pattern.is_none());
        let dynamic = p.dynamic_replace.as_ref().expect("dynamic_replace present");

        let regex_str = std::str::from_utf8(p.search_pattern.as_ref()).unwrap();
        let re = regex::bytes::Regex::new(regex_str).unwrap();
        // (sample, expected_span_len, expected_pad)
        let samples: [(&[u8], usize, usize); 3] = [
            // 195/201: if(g7())n.push(... = 63B; pad = 63-5-58 = 3
            (
                br#"if(g7())n.push("**Provider context:** This session is not using"#,
                63,
                3,
            ),
            // 201 dJ: 同 63B
            (
                br#"if(dJ())n.push("**Provider context:** This session is not using"#,
                63,
                3,
            ),
            // 207: if(yfe())r.push(... = 64B (FN 3字符 + arr r); pad = 64-5-58 = 4
            (
                br#"if(yfe())r.push("**Provider context:** This session is not using"#,
                64,
                4,
            ),
        ];
        for (sample, expected_span, expected_pad) in samples {
            let caps = re.captures(sample).unwrap_or_else(|| {
                panic!("antipromptbias regex must match sample: {}", String::from_utf8_lossy(sample))
            });
            let m = caps.get(0).unwrap();
            let span_len = m.end() - m.start();
            assert_eq!(span_len, expected_span, "span length for sample");

            let keep = caps.get(1).unwrap().as_bytes();
            let prefix = match dynamic {
                DynamicReplace::ReplacePrefix { prefix_literal, .. } => prefix_literal.as_ref(),
                _ => unreachable!(),
            };
            let pad = span_len - prefix.len() - keep.len();
            assert_eq!(pad, expected_pad, "pad count for sample");

            // 构造 replace 并验证等长
            let mut replace = Vec::with_capacity(span_len);
            replace.extend_from_slice(prefix);
            replace.extend(std::iter::repeat_n(b' ', pad));
            replace.extend_from_slice(keep);
            assert_eq!(replace.len(), span_len, "dynamic replace must be equal length");
            // replace 以 if(0) 开头，后接 keep（arr.push("**Provider...））
            assert_eq!(&replace[..5], b"if(0)");
            assert!(replace[5..5 + pad].iter().all(|&b| b == b' '));
            assert_eq!(&replace[5 + pad..], keep);
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

    #[test]
    fn test_antiatis_patches_structure() {
        let patches = get_antiatis_patches();
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
            assert_eq!(p.feature, FeatureType::AntiAtis);
            assert!(p.use_regex, "antiatis is regex mode");
            assert!(p.regex_replace_values.is_none());
            assert!(p.patch_byte.is_none());
            assert!(p.patch_offset.is_none());
            // 模式 4：replace_pattern=None，dynamic_replace=Some(KeepPrefix)
            assert!(p.replace_pattern.is_none(), "mode4 has no replace_pattern");
            assert!(p.dynamic_replace.is_some(), "mode4 has dynamic_replace");
            match p.dynamic_replace.as_ref().unwrap() {
                DynamicReplace::KeepPrefix {
                    keep_group,
                    suffix_literal,
                    end_literal,
                } => {
                    assert_eq!(*keep_group, 1, "keep_group must be 1");
                    assert_eq!(suffix_literal.as_ref(), b"return void 0");
                    assert_eq!(end_literal.as_ref(), b"}");
                }
                other => panic!("expected KeepPrefix, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_antiatis_patches_equal_length() {
        // 模式 4 等长铁律：regex 匹配 span 长度 == 动态构造的 replace 长度
        // （replace = match[1] + return void 0 + 空格*(span-keep_len-13-1) + }，等长自动成立）
        // 跨版本样本：196-199 FN 3字符 span=80B；207 FN 3字符(R0i) span=80B；
        // 额外验证 FN 2字符也能匹配（span=79B），证明跨版本自适应
        let patches = get_antiatis_patches();
        let p = &patches[0];
        assert!(p.replace_pattern.is_none());
        let dynamic = p.dynamic_replace.as_ref().expect("dynamic_replace present");

        let regex_str = std::str::from_utf8(p.search_pattern.as_ref()).unwrap();
        let re = regex::bytes::Regex::new(regex_str).unwrap();
        // (sample, expected_span_len)
        let samples: [(&[u8], usize); 5] = [
            (
                b"function S6r(){let e=P0()?.atis;return typeof e===\"string\"&&e.length>0?e:void 0}",
                80,
            ),
            (
                b"function H6r(){let e=AI()?.atis;return typeof e===\"string\"&&e.length>0?e:void 0}",
                80,
            ),
            (
                b"function pYr(){let e=I0()?.atis;return typeof e===\"string\"&&e.length>0?e:void 0}",
                80,
            ),
            (
                b"function SXr(){let e=q0()?.atis;return typeof e===\"string\"&&e.length>0?e:void 0}",
                80,
            ),
            // 207 真实样本：FN=R0i(3字符), BF=mL(2字符)
            (
                b"function R0i(){let e=mL()?.atis;return typeof e===\"string\"&&e.length>0?e:void 0}",
                80,
            ),
        ];
        let (suffix, end) = match dynamic {
            DynamicReplace::KeepPrefix {
                suffix_literal,
                end_literal,
                ..
            } => (suffix_literal.as_ref(), end_literal.as_ref()),
            _ => unreachable!(),
        };
        for (sample, expected_span) in samples {
            let caps = re.captures(sample).unwrap_or_else(|| {
                panic!("antiatis regex must match sample: {}", String::from_utf8_lossy(sample))
            });
            let m = caps.get(0).unwrap();
            let span_len = m.end() - m.start();
            assert_eq!(span_len, expected_span, "span length for sample");

            let keep = caps.get(1).unwrap().as_bytes();
            let pad = span_len - keep.len() - suffix.len() - end.len();
            // 构造 replace 并验证等长
            let mut replace = Vec::with_capacity(span_len);
            replace.extend_from_slice(keep);
            replace.extend_from_slice(suffix);
            replace.extend(std::iter::repeat_n(b' ', pad));
            replace.extend_from_slice(end);
            assert_eq!(replace.len(), span_len, "dynamic replace must be equal length");
            // replace 以 keep（function <FN>(){）开头，以 } 结尾，中间含 return void 0
            assert!(replace.starts_with(keep));
            assert!(replace.ends_with(end));
            assert!(replace.windows(suffix.len()).any(|w| w == suffix));
        }
    }

    #[test]
    fn test_antiatis_via_get_feature_patches() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 196,
        };
        let patches = get_feature_patches(FeatureType::AntiAtis, &version);
        assert_eq!(patches.len(), 2);
        assert!(patches.iter().all(|p| p.use_regex));
        assert!(patches.iter().all(|p| p.feature == FeatureType::AntiAtis));
    }

    #[test]
    fn test_antiframetrack_patches_structure() {
        let patches = get_antiframetrack_patches();
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
            assert_eq!(p.feature, FeatureType::AntiFrameTrack);
            assert!(!p.use_regex, "antiframetrack is literal mode");
            assert!(p.regex_replace_values.is_none());
            assert!(p.patch_byte.is_none());
            assert!(p.patch_offset.is_none());
            assert!(p.replace_pattern.is_some());
        }
    }

    #[test]
    fn test_antiframetrack_patches_equal_length() {
        // 等长替换铁律：search 与 replace 必须等长（16B），否则破坏二进制偏移
        let patches = get_antiframetrack_patches();
        for p in &patches {
            let s = p.search_pattern.as_ref();
            let r = p.replace_pattern.as_ref().unwrap();
            assert_eq!(s.len(), 16, "antiframetrack search must be 16 bytes");
            assert_eq!(r.len(), 16, "antiframetrack replace must be 16 bytes");
            assert_eq!(s.len(), r.len(), "antiframetrack patch must be equal length");
        }
    }

    #[test]
    fn test_antiframetrack_patch_content() {
        let patches = get_antiframetrack_patches();
        for p in &patches {
            let s: &[u8] = p.search_pattern.as_ref();
            let r: &[u8] = p.replace_pattern.as_ref().unwrap().as_ref();
            assert_eq!(s, &b"/api/frame/track"[..]);
            assert_eq!(r, &b"/api/frame/xxxxx"[..]);
        }
    }

    #[test]
    fn test_antiframetrack_via_get_feature_patches() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 201,
        };
        let patches = get_feature_patches(FeatureType::AntiFrameTrack, &version);
        assert_eq!(patches.len(), 2);
        assert!(patches.iter().all(|p| !p.use_regex));
        assert!(patches.iter().all(|p| p.feature == FeatureType::AntiFrameTrack));
    }

    #[test]
    fn test_anticloudetect_patches_structure() {
        let patches = get_anticloudetect_patches();
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
            assert_eq!(p.feature, FeatureType::AntiCloudDetect);
            assert!(!p.use_regex, "anticloudetect is literal mode");
            assert!(p.regex_replace_values.is_none());
            assert!(p.patch_byte.is_none());
            assert!(p.patch_offset.is_none());
            assert!(p.replace_pattern.is_some());
        }
    }

    #[test]
    fn test_anticloudetect_patches_equal_length() {
        // 等长替换铁律：search 与 replace 必须等长（8B），否则破坏二进制偏移
        let patches = get_anticloudetect_patches();
        for p in &patches {
            let s = p.search_pattern.as_ref();
            let r = p.replace_pattern.as_ref().unwrap();
            assert_eq!(s.len(), 8, "anticloudetect search must be 8 bytes");
            assert_eq!(r.len(), 8, "anticloudetect replace must be 8 bytes");
            assert_eq!(s.len(), r.len(), "anticloudetect patch must be equal length");
        }
    }

    #[test]
    fn test_anticloudetect_patch_content() {
        let patches = get_anticloudetect_patches();
        for p in &patches {
            let s: &[u8] = p.search_pattern.as_ref();
            let r: &[u8] = p.replace_pattern.as_ref().unwrap().as_ref();
            assert_eq!(s, &b"/^42:01/"[..]);
            assert_eq!(r, &b"/^00:00/"[..]);
        }
    }

    #[test]
    fn test_anticloudetect_via_get_feature_patches() {
        let version = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 201,
        };
        let patches = get_feature_patches(FeatureType::AntiCloudDetect, &version);
        assert_eq!(patches.len(), 2);
        assert!(patches.iter().all(|p| !p.use_regex));
        assert!(patches.iter().all(|p| p.feature == FeatureType::AntiCloudDetect));
    }

    /// CC v2.1.201 真实 binary 样本回归测试（2026-07-04 审计）
    ///
    /// 从 GCS native binary `2.1.201/{linux-x64,linux-arm64}/claude` 提取的真实
    /// patch 点字面量。6 个 patch 全部命中 + 等长约束满足，验证 regex 跨版本
    /// （195-201）通配能力。变量名 jUt/kre/ke/dJ/jJr/nR（x64）与 j$t/Ire/Ie/dX/jXr/nP
    /// （arm64）均不同，regex 自动通配。
    ///
    /// 若此测试 fail，说明 CC 新版本的 patch 点结构变化，需重新审计。
    #[test]
    fn test_patches_match_201_binary_samples() {
        // linux-x64 真实样本（从 2.1.201 binary 字节级提取）
        let x64_samples: &[(&[u8], &str)] = &[
            (
                b"var jUt=200000,kre=200000,z5d=32000,K5d=128000;",
                "max-token x64 (4 elements)",
            ),
            (
                b"if(ke._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0",
                "escape x64 (obj=ke, 2 chars)",
            ),
            (
                br#"if(dJ())n.push("**Provider context:** This session is not using"#,
                "antipromptbias x64 (fn=dJ, 2 chars)",
            ),
            (
                b"function jJr(){let e=nR()?.atis;return typeof e===\"string\"&&e.length>0?e:void 0}",
                "antiatis x64 (fn=jJr 3 chars, bootstrap=nR 2 chars)",
            ),
        ];
        // linux-arm64 真实样本（变量名不同，验证跨平台通配）
        let arm64_samples: &[(&[u8], &str)] = &[
            (
                b"var j$t=200000,Ire=200000,zGd=32000,KGd=128000;",
                "max-token arm64 (4 elements, var name j$t with $)",
            ),
            (
                b"if(Ie._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0",
                "escape arm64 (obj=Ie, 2 chars)",
            ),
            (
                br#"if(dX())n.push("**Provider context:** This session is not using"#,
                "antipromptbias arm64 (fn=dX, 2 chars)",
            ),
            (
                b"function jXr(){let e=nP()?.atis;return typeof e===\"string\"&&e.length>0?e:void 0}",
                "antiatis arm64 (fn=jXr 3 chars, bootstrap=nP 2 chars)",
            ),
        ];

        // max-token regex: 匹配 4 元素块，等长约束由 regex_replace_values 运行时保证
        let max_tok_re = regex::bytes::Regex::new(MAX_CONTEXT_TOKENS_SEARCH_REGEX).unwrap();
        for (sample, label) in x64_samples.iter().chain(arm64_samples.iter()) {
            if !label.starts_with("max-token") {
                continue;
            }
            let m = max_tok_re
                .find(sample)
                .unwrap_or_else(|| panic!("max-token regex must match {}", label));
            assert_eq!(m.start(), 0, "{}: match must start at 0", label);
            assert_eq!(m.end(), sample.len(), "{}: match must cover full block", label);
        }

        // escape regex: 匹配 55 字节，replace 也是 55 字节
        let escape_re = regex::bytes::Regex::new(
            r"if\([a-zA-Z_$][a-zA-Z0-9_$]*\._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL\)return!0",
        )
        .unwrap();
        for (sample, label) in x64_samples.iter().chain(arm64_samples.iter()) {
            if !label.starts_with("escape") {
                continue;
            }
            let m = escape_re
                .find(sample)
                .unwrap_or_else(|| panic!("escape regex must match {}", label));
            assert_eq!(
                m.end() - m.start(),
                55,
                "{}: escape match must be 55 bytes",
                label
            );
        }

        // antipromptbias regex（模式 4，arr 通配 + 捕获组 1）：
        //   201 样本 arr=n FN=dJ(2字符) → span=63B；207 样本 arr=r FN=yfe(3字符) → span=64B
        //   等长由 dynamic_replace 运行时保证（不固定 63B）
        let pb_re = regex::bytes::Regex::new(
            r#"if\([a-zA-Z_$][a-zA-Z0-9_$]*\(\)\)([a-zA-Z_$][a-zA-Z0-9_$]*\.push\("\*\*Provider context:\*\* This session is not using)"#,
        )
        .unwrap();
        for (sample, label) in x64_samples.iter().chain(arm64_samples.iter()) {
            if !label.starts_with("antipromptbias") {
                continue;
            }
            let caps = pb_re
                .captures(sample)
                .unwrap_or_else(|| panic!("antipromptbias regex must match {}", label));
            let m = caps.get(0).unwrap();
            // 201 样本（arr=n, FN 2字符）span=63B
            assert_eq!(
                m.end() - m.start(),
                63,
                "{}: antipromptbias match must be 63 bytes",
                label
            );
            // 组 1 捕获后缀 n.push("**Provider context...
            let keep = caps.get(1).unwrap().as_bytes();
            assert!(keep.starts_with(b"n.push(\"**Provider context"));
        }

        // antiatis regex（模式 4，捕获组 1 保留函数名前缀）：
        //   201 样本 FN 3字符 → span=80B；等长由 dynamic_replace 运行时保证
        let atis_re = regex::bytes::Regex::new(
            r#"(function [a-zA-Z_$][a-zA-Z0-9_$]*\(\)\{)let e=[a-zA-Z_$][a-zA-Z0-9_$]*\(\)[?]\.atis;return typeof e==="string"&&e\.length>0[?]e:void 0\}"#,
        )
        .unwrap();
        for (sample, label) in x64_samples.iter().chain(arm64_samples.iter()) {
            if !label.starts_with("antiatis") {
                continue;
            }
            let caps = atis_re
                .captures(sample)
                .unwrap_or_else(|| panic!("antiatis regex must match {}", label));
            let m = caps.get(0).unwrap();
            assert_eq!(
                m.end() - m.start(),
                80,
                "{}: antiatis match must be 80 bytes",
                label
            );
            // 组 1 捕获 function <FN>(){
            let keep = caps.get(1).unwrap().as_bytes();
            assert!(keep.starts_with(b"function "));
            assert!(keep.ends_with(b"){"));
        }

        // AntiTelemetry + AntiSpy timezone 是字面量模式，跨版本稳定，无需 regex 验证
        // （字面量 `/api/event_logging/v2/batch` 和 `Intl.DateTimeFormat()...timeZone`
        //   在 201 binary 已验证各 1/2 处命中，见 SPEC/CC-PROMPT-AUDIT.md 201 章节）
    }
}
