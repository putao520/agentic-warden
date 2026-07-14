//! 文件补丁实现
//!
//! 直接修改磁盘上的 AI CLI 文件，支持 npm 安装（JS）和本地二进制安装（ELF/Mach-O）
//! 共享层：apply_file_patch 引擎 + regex 辅助 + 备份/恢复 + magic bytes 分类。

use crate::patcher::types::{
    DynamicReplace, UnifiedPatchError, UnifiedPatchPattern, UnifiedPatchResult, Result,
};
use std::fs;
use std::path::{Path, PathBuf};

/// 在匹配文本中按顺序替换 `200000` 字面量为 `regex_replace_values` 中的值
///
/// 每出现一次 `200000`，消耗 `regex_replace_values` 的下一个值并替换为
/// 该 6 位数值的 ASCII 字节（等长替换）。多余的 `200000` 保留不动。
fn apply_regex_replace(matched: &[u8], replace_values: &[u32]) -> Vec<u8> {
    use crate::patcher::claude::versions::encode_max_context_tokens;
    let needle = b"200000";
    let mut out = Vec::with_capacity(matched.len());
    let mut val_iter = replace_values.iter();
    let mut i = 0;
    while i + needle.len() <= matched.len() {
        if &matched[i..i + needle.len()] == needle {
            if let Some(&v) = val_iter.next() {
                let enc = encode_max_context_tokens(v);
                out.extend_from_slice(&enc);
            } else {
                out.extend_from_slice(needle);
            }
            i += needle.len();
        } else {
            out.push(matched[i]);
            i += 1;
        }
    }
    out.extend_from_slice(&matched[i..]);
    out
}

/// NativeBinary 补丁前自动备份（仅首次，且仅对二进制文件）。
/// 非二进制文件无备份动作。
fn ensure_binary_backup(file_path: &Path) -> Result<()> {
    let backup_path = backup_path_for(file_path);
    if !backup_path.exists() && is_binary_file(file_path) {
        fs::copy(file_path, &backup_path).map_err(|e| {
            UnifiedPatchError::FileError(std::io::Error::new(
                e.kind(),
                format!("备份失败 {}: {}", file_path.display(), e),
            ))
        })?;
    }
    Ok(())
}

/// 备份路径 SSOT：`<file>.aiw-backup`。
///
/// 统一备份路径格式，消除 `ensure_binary_backup` 与 `restore_from_backup`
/// 各自拼接 `format!("{}.aiw-backup", ...)` 的重复。
fn backup_path_for(file_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.aiw-backup", file_path.display()))
}

/// 该文件是否已存在 `.aiw-backup` 备份（即已被 patch 且未 restore）。
///
/// 复用 `backup_path_for` SSOT，避免调用方自行拼路径（`with_extension` 语义不同，
/// 对含 `.` 的文件名会产生路径错位）。patch 状态检测用此判定。
pub(crate) fn backup_exists(file_path: &Path) -> bool {
    backup_path_for(file_path).exists()
}

/// 在 `content` 中收集 `search` 的所有起始偏移（无重叠）。
fn find_all_literal_positions(content: &[u8], search: &[u8]) -> Vec<usize> {
    let search_len = search.len();
    let mut positions = Vec::new();
    let mut start = 0;
    while start + search_len <= content.len() {
        if let Some(pos) = content[start..]
            .windows(search_len)
            .position(|window| window == search)
        {
            positions.push(start + pos);
            start = start + pos + search_len;
        } else {
            break;
        }
    }
    positions
}

/// 构造「文件已补丁」结果（统一返回值构造，消除 `apply_file_patch` 与
/// `apply_regex_file_patch` 重复构造 `UnifiedPatchResult::FilePatched`）。
fn file_patched_result(file_path: &Path) -> UnifiedPatchResult {
    UnifiedPatchResult::FilePatched {
        path: file_path.display().to_string(),
    }
}

/// 用 `replace`（整段覆盖）从后往前替换 `positions` 处的匹配，等长替换。
fn apply_replace_to_positions(
    content: &[u8],
    positions: &[usize],
    search_len: usize,
    replace: &[u8],
) -> Vec<u8> {
    let mut new_content = content.to_vec();
    for &pos in positions.iter().rev() {
        let mut result = Vec::with_capacity(new_content.len());
        result.extend_from_slice(&new_content[..pos]);
        result.extend_from_slice(replace);
        result.extend_from_slice(&new_content[pos + search_len..]);
        new_content = result;
    }
    new_content
}

/// 用单字节 `patch_byte`（偏移 `offset`）替换所有匹配位置。
fn apply_patch_byte_to_positions(
    content: &[u8],
    positions: &[usize],
    patch_byte: u8,
    offset: usize,
) -> Vec<u8> {
    let mut new_content = content.to_vec();
    for &pos in positions {
        new_content[pos + offset] = patch_byte;
    }
    new_content
}

/// 应用文件补丁
pub fn apply_file_patch(
    file_path: &Path,
    pattern: &UnifiedPatchPattern,
) -> Result<UnifiedPatchResult> {
    ensure_binary_backup(file_path)?;

    // 读取文件内容
    let content = fs::read(file_path)?;

    if pattern.use_regex {
        return apply_regex_file_patch(file_path, &content, pattern);
    }

    // 字面量模式：查找所有匹配位置
    let search = pattern.search_pattern.as_ref();
    let positions = find_all_literal_positions(&content, search);
    if positions.is_empty() {
        return Err(UnifiedPatchError::PatternNotFound(format!(
            "{:?}",
            pattern.search_pattern
        )));
    }

    // 应用补丁（替换所有匹配）
    let patched_content = match (pattern.replace_pattern.as_ref(), pattern.patch_byte, pattern.patch_offset) {
        (Some(replace), _, _) => {
            apply_replace_to_positions(&content, &positions, search.len(), replace.as_ref())
        }
        (None, Some(patch_byte), Some(offset)) => {
            apply_patch_byte_to_positions(&content, &positions, patch_byte, offset)
        }
        (None, _, _) => {
            return Err(UnifiedPatchError::PatternNotFound(
                "No replacement pattern or patch byte specified".to_string(),
            ));
        }
    };

    write_back(file_path, &patched_content)?;

    Ok(file_patched_result(file_path))
}

/// regex 字面量替换模式：用 `replace_pattern` 整段覆盖每个匹配文本（要求等长）。
/// 从后往前应用以保持偏移稳定。
fn apply_regex_literal_replace(
    content: &[u8],
    matches: &[regex::bytes::Match<'_>],
    replace: &[u8],
) -> Result<Vec<u8>> {
    let mut new_content = content.to_vec();
    for m in matches.iter().rev() {
        let span_len = m.end() - m.start();
        if replace.len() != span_len {
            return Err(UnifiedPatchError::Other(format!(
                "regex literal patch must be equal length: match={}, replace={}",
                span_len,
                replace.len()
            )));
        }
        new_content[m.start()..m.end()].copy_from_slice(replace);
    }
    Ok(new_content)
}

/// regex 数字替换模式：按 `regex_replace_values` 替换每个匹配中的 200000
/// （等长 6 位）。长度异常时用 splice 兜底。
fn apply_regex_numeric_replace(
    content: &[u8],
    matches: &[regex::bytes::Match<'_>],
    replace_values: &[u32],
) -> Vec<u8> {
    let mut new_content = content.to_vec();
    for m in matches.iter().rev() {
        let matched_bytes = &content[m.start()..m.end()];
        let replaced = apply_regex_replace(matched_bytes, replace_values);
        let span_len = m.end() - m.start();
        if replaced.len() == span_len {
            new_content[m.start()..m.end()].copy_from_slice(&replaced);
        } else {
            new_content.splice(m.start()..m.end(), replaced);
        }
    }
    new_content
}

/// regex 动态替换模式（模式 4）：保留捕获组 + 字面量 + 空格填充至等长
///
/// 对每个 capture（从后往前，保持偏移稳定），按 `DynamicReplace` variant
/// 构造等长 replace：
/// - `ReplacePrefix`: `prefix_literal` + 空格填充 + `match[keep_group]`
/// - `KeepPrefix`: `match[keep_group]` + `suffix_literal` + 空格填充 + `end_literal`
///
/// 空格数 = `span_len - 固定部分`，自动适应 minified 变量名长度跨版本变化，
/// 等长约束自动成立。单次调用处理单个 capture（调用方在循环里从后往前传入）。
fn apply_regex_dynamic_replace(
    content: &[u8],
    captures: &regex::bytes::Captures<'_>,
    dynamic_replace: &DynamicReplace,
) -> Result<Vec<u8>> {
    // 取整个匹配 span
    let m = captures
        .get(0)
        .expect("capture group 0 always exists");
    let span_len = m.end() - m.start();

    // 按 variant 构造等长 replace（keep_group 从 destructure 取，避免跨 variant 重复取）
    let replace = match dynamic_replace {
        DynamicReplace::ReplacePrefix {
            keep_group,
            prefix_literal,
        } => {
            let keep = captures
                .get(*keep_group)
                .ok_or_else(|| {
                    UnifiedPatchError::Other(format!(
                        "dynamic replace keep_group {} not captured",
                        keep_group
                    ))
                })?
                .as_bytes();
            let prefix_len = prefix_literal.len();
            let pad = span_len
                .checked_sub(prefix_len + keep.len())
                .ok_or_else(|| {
                    UnifiedPatchError::Other(format!(
                        "ReplacePrefix span too short: span={}, prefix={}, keep={}",
                        span_len,
                        prefix_len,
                        keep.len()
                    ))
                })?;
            let mut r = Vec::with_capacity(span_len);
            r.extend_from_slice(prefix_literal);
            r.extend(std::iter::repeat_n(b' ', pad));
            r.extend_from_slice(keep);
            r
        }
        DynamicReplace::KeepPrefix {
            keep_group,
            suffix_literal,
            end_literal,
        } => {
            let keep = captures
                .get(*keep_group)
                .ok_or_else(|| {
                    UnifiedPatchError::Other(format!(
                        "dynamic replace keep_group {} not captured",
                        keep_group
                    ))
                })?
                .as_bytes();
            let suffix_len = suffix_literal.len();
            let end_len = end_literal.len();
            let pad = span_len
                .checked_sub(keep.len() + suffix_len + end_len)
                .ok_or_else(|| {
                    UnifiedPatchError::Other(format!(
                        "KeepPrefix span too short: span={}, keep={}, suffix={}, end={}",
                        span_len,
                        keep.len(),
                        suffix_len,
                        end_len
                    ))
                })?;
            let mut r = Vec::with_capacity(span_len);
            r.extend_from_slice(keep);
            r.extend_from_slice(suffix_literal);
            r.extend(std::iter::repeat_n(b' ', pad));
            r.extend_from_slice(end_literal);
            r
        }
    };
    debug_assert_eq!(
        replace.len(),
        span_len,
        "dynamic replace must be equal length"
    );

    // 覆盖（单次，调用方在循环里从后往前处理多个 captures）
    let mut new = content.to_vec();
    new[m.start()..m.end()].copy_from_slice(&replace);
    Ok(new)
}

/// regex 模式的文件补丁
///
/// 三种子模式（见 `UnifiedPatchPattern.replace_pattern` 文档）：
/// - `replace_pattern=Some`：regex 字面量替换模式，regex 匹配后用
///   `replace_pattern` 整段覆盖匹配文本（要求等长，跨版本 patch 点用）。
/// - `replace_pattern=None` + `regex_replace_values=Some`：regex 数字替换模式，
///   扫描所有匹配，按 `regex_replace_values` 替换其中的 200000 数字字面量（等长 6 位）。
/// - `replace_pattern=None` + `regex_replace_values=None` + `dynamic_replace=Some`：
///   regex 动态替换模式（模式 4），用 `captures_iter` 收集捕获组，保留指定组内容，
///   其余用字面量 + 空格填充至等长（跨版本自适应 minified 变量名长度变化）。
fn apply_regex_file_patch(
    file_path: &Path,
    content: &[u8],
    pattern: &UnifiedPatchPattern,
) -> Result<UnifiedPatchResult> {
    let regex_str = std::str::from_utf8(pattern.search_pattern.as_ref())
        .map_err(|e| UnifiedPatchError::Other(format!("invalid regex utf-8: {}", e)))?;
    let re = regex::bytes::Regex::new(regex_str)
        .map_err(|e| UnifiedPatchError::Other(format!("invalid regex: {}", e)))?;

    // 模式 4：regex 动态替换（需 captures_iter 取捕获组，从后往前应用以保持偏移稳定）
    if let Some(dynamic_replace) = pattern.dynamic_replace.as_ref() {
        let captures: Vec<_> = re.captures_iter(content).collect();
        if captures.is_empty() {
            return Err(UnifiedPatchError::PatternNotFound(format!(
                "regex {:?} did not match",
                regex_str
            )));
        }
        let mut new_content = content.to_vec();
        for cap in captures.iter().rev() {
            new_content = apply_regex_dynamic_replace(&new_content, cap, dynamic_replace)?;
        }
        write_back(file_path, &new_content)?;
        return Ok(file_patched_result(file_path));
    }

    // 收集所有匹配（从后往前应用以保持偏移稳定）
    let matches: Vec<_> = re.find_iter(content).collect();
    if matches.is_empty() {
        return Err(UnifiedPatchError::PatternNotFound(format!(
            "regex {:?} did not match",
            regex_str
        )));
    }

    let new_content = match pattern.replace_pattern.as_ref() {
        Some(replace) => apply_regex_literal_replace(content, &matches, replace.as_ref())?,
        None => {
            let replace_values = pattern.regex_replace_values.clone().unwrap_or_default();
            apply_regex_numeric_replace(content, &matches, &replace_values)
        }
    };

    write_back(file_path, &new_content)?;

    Ok(file_patched_result(file_path))
}

/// 写回文件（对二进制文件使用 rename 策略避免 "Text file busy"）
fn write_back(file_path: &Path, content: &[u8]) -> Result<()> {
    if is_binary_file(file_path) {
        let tmp_path = PathBuf::from(format!("{}.aiw-tmp", file_path.display()));
        fs::write(&tmp_path, content)?;
        #[cfg(unix)]
        {
            if let Ok(metadata) = fs::metadata(file_path) {
                let _ = fs::set_permissions(&tmp_path, metadata.permissions());
            }
        }
        fs::rename(&tmp_path, file_path)?;
    } else {
        fs::write(file_path, content)?;
    }
    Ok(())
}

/// 检查文件是否已应用补丁
pub fn is_file_patched(
    file_path: &Path,
    pattern: &UnifiedPatchPattern,
) -> Result<bool> {
    let content = fs::read(file_path)?;

    if pattern.use_regex {
        return is_file_patched_regex(&content, pattern);
    }

    let search = pattern.search_pattern.as_ref();
    // 检查是否包含原始模式（未补丁）
    let has_original = content.windows(search.len()).any(|window| window == search);

    Ok(!has_original)
}

/// regex 模式下的「已补丁」判定
///
/// 三种子模式：
/// - `replace_pattern=Some`（regex 字面量替换模式）：匹配到的文本等于
///   `replace_pattern` 则已补丁，否则未补丁。
/// - `replace_pattern=None` + `regex_replace_values=Some`（regex 数字替换模式）：
///   匹配到的文本里是否还有 200000。未补丁 = 含 200000；已补丁 = 不含 200000。
/// - `replace_pattern=None` + `regex_replace_values=None` + `dynamic_replace=Some`
///   （regex 动态替换模式，模式 4）：patched 后 regex 不再匹配（如 `if(0   )`
///   不匹配 `if\([a-zA-Z_$]...\)` 因 `0` 不满足首字符类）。regex 能匹配 = 未补丁。
fn is_file_patched_regex(content: &[u8], pattern: &UnifiedPatchPattern) -> Result<bool> {
    let regex_str = std::str::from_utf8(pattern.search_pattern.as_ref())
        .map_err(|e| UnifiedPatchError::Other(format!("invalid regex utf-8: {}", e)))?;
    let re = regex::bytes::Regex::new(regex_str)
        .map_err(|e| UnifiedPatchError::Other(format!("invalid regex: {}", e)))?;

    if let Some(replace) = pattern.replace_pattern.as_ref() {
        // regex 字面量替换模式：所有匹配都等于 replace_pattern 才算已补丁
        let has_unpatched = re
            .find_iter(content)
            .any(|m| m.as_bytes() != replace.as_ref());
        return Ok(!has_unpatched);
    }

    // 模式 4：regex 动态替换 — regex 能匹配 = 未补丁（patched 后不再匹配原模式）
    if pattern.dynamic_replace.is_some() {
        let has_unpatched = re.find_iter(content).any(|_| true);
        return Ok(!has_unpatched);
    }

    // regex 数字替换模式：未补丁的匹配仍含 200000
    let has_unpatched = re
        .find_iter(content)
        .any(|m| m.as_bytes().windows(6).any(|w| w == b"200000"));

    Ok(!has_unpatched)
}

/// 文件头 4 字节 magic 分类（统一 magic bytes SSOT，消除 `detect_installation`
/// 与 `is_binary_file` 的重复匹配）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicKind {
    /// ELF / Mach-O 原生二进制
    NativeBinary,
    /// `#!` shebang（npm shell 脚本）
    Shebang,
    /// 未知 / 无法识别
    Unknown,
}

/// 按 magic bytes 前 4 字节分类文件类型。
///
/// - ELF: `7f 45 4c 46`
/// - Mach-O: `fe ed fa ce/cf` 及小端反转 `ce/cf fa ed fe`
/// - Shebang: `23 21` (`#!`)
pub fn classify_magic_bytes(header: &[u8]) -> MagicKind {
    if header.len() < 4 {
        return MagicKind::Unknown;
    }
    match &header[..4] {
        // ELF magic: 0x7f 'E' 'L' 'F'
        [0x7f, 0x45, 0x4c, 0x46] => MagicKind::NativeBinary,
        // Mach-O magic (32-bit and 64-bit, both endianness)
        [0xfe, 0xed, 0xfa, 0xce]
        | [0xfe, 0xed, 0xfa, 0xcf]
        | [0xce, 0xfa, 0xed, 0xfe]
        | [0xcf, 0xfa, 0xed, 0xfe] => MagicKind::NativeBinary,
        // Shebang (#!) — npm shell 脚本
        [0x23, 0x21, ..] => MagicKind::Shebang,
        _ => MagicKind::Unknown,
    }
}

/// 检测文件是否为二进制文件（通过 magic bytes）
pub fn is_binary_file(path: &Path) -> bool {
    fs::read(path)
        .map(|data| classify_magic_bytes(&data) == MagicKind::NativeBinary)
        .unwrap_or(false)
}

/// 从备份恢复文件
pub fn restore_from_backup(file_path: &Path) -> Result<()> {
    let backup_path = backup_path_for(file_path);
    if !backup_path.exists() {
        return Err(UnifiedPatchError::FileError(
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("备份文件不存在: {}", backup_path.display()),
            ),
        ));
    }
    fs::copy(&backup_path, file_path)?;
    fs::remove_file(&backup_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patcher::types::PatchType;
    use crate::patcher::claude::versions::{ClaudeVersion, MAX_CONTEXT_TOKENS_SEARCH_REGEX};
    use std::borrow::Cow;

    #[test]
    fn test_regex_replace_swaps_200000() {
        let matched = b"var YOt=200000,Pte=200000,Evi=20000,Wkd=32000,qkd=128000;";
        let out = apply_regex_replace(matched, &[500000, 500000]);
        let s = String::from_utf8(out).unwrap();
        assert_eq!(
            s,
            "var YOt=500000,Pte=500000,Evi=20000,Wkd=32000,qkd=128000;"
        );
        // 等长替换
        assert_eq!(s.len(), matched.len());
    }

    #[test]
    fn test_apply_regex_file_patch_writes_target_values() {
        // 构造临时二进制内容并验证 regex 文件补丁逻辑
        let tmp = std::env::temp_dir().join("aiw_regex_patch_test.bin");
        let original = b"...upperLimit-1}var YOt=200000,Pte=200000,Evi=20000,Wkd=32000,qkd=128000;var BE=E(()=>{";
        std::fs::write(&tmp, original).unwrap();

        let pattern = UnifiedPatchPattern {
            feature: crate::patcher::types::FeatureType::MaxContextTokens,
            patch_type: PatchType::File,
            search_pattern: Cow::Borrowed(MAX_CONTEXT_TOKENS_SEARCH_REGEX.as_bytes()),
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test"),
            use_regex: true,
            regex_replace_values: Some(vec![500000, 300000]),
            dynamic_replace: None,
        };

        let res = apply_file_patch(&tmp, &pattern);
        assert!(res.is_ok(), "{:?}", res);

        let patched = std::fs::read(&tmp).unwrap();
        let s = String::from_utf8_lossy(&patched);
        assert!(s.contains("var YOt=500000,Pte=300000,Evi=20000,Wkd=32000,qkd=128000;"));
        // 等长：总长度不变
        assert_eq!(patched.len(), original.len());

        // 已补丁判定
        let patched_flag = is_file_patched(&tmp, &pattern).unwrap();
        assert!(patched_flag);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_get_max_context_tokens_patches_via_registry() {
        let v = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 195,
        };
        let patches = crate::patcher::claude::registry::get_max_context_tokens_patches(&v, 500000, 500000);
        assert_eq!(patches.len(), 2);
    }

    #[test]
    fn test_apply_regex_file_patch_literal_replace() {
        // regex 字面量替换模式（use_regex=true + replace_pattern=Some）：
        // 用 AntiPromptBias 的 pattern 验证 apply_file_patch + is_file_patched
        let tmp = std::env::temp_dir().join("aiw_regex_literal_patch_test.bin");
        // 198 样本：if(dX())n.push("**Provider context:** This session is not using
        let original: Vec<u8> = [
            &b"...prefix..."[..],
            br#"if(dX())n.push("**Provider context:** This session is not using 3P."#,
            &b"...suffix..."[..],
        ]
        .concat();
        std::fs::write(&tmp, &original).unwrap();

        let pattern = UnifiedPatchPattern {
            feature: crate::patcher::types::FeatureType::AntiPromptBias,
            patch_type: PatchType::File,
            search_pattern: Cow::Borrowed(
                br#"if\([a-zA-Z_$][a-zA-Z0-9_$]*\(\)\)n\.push\("\*\*Provider context:\*\* This session is not using"#,
            ),
            replace_pattern: Some(Cow::Borrowed(
                br#"if(0   )n.push("**Provider context:** This session is not using"#,
            )),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test regex literal replace"),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: None,
        };

        // 未补丁前：is_file_patched 返回 false
        assert!(!is_file_patched(&tmp, &pattern).unwrap());

        let res = apply_file_patch(&tmp, &pattern);
        assert!(res.is_ok(), "{:?}", res);

        let patched = std::fs::read(&tmp).unwrap();
        // 等长：总长度不变（63B → 63B）
        assert_eq!(patched.len(), original.len());
        // 已替换为 if(0   )
        assert!(String::from_utf8_lossy(&patched).contains("if(0   )n.push"));
        // 原 if(dX()) 已不存在
        assert!(!String::from_utf8_lossy(&patched).contains("if(dX())"));

        // 已补丁判定：true
        assert!(is_file_patched(&tmp, &pattern).unwrap());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_apply_regex_file_patch_literal_replace_antispy_escape() {
        // 逃生口 patch regex 字面量替换：if(Oe.xxx)return!0 → if(1) + 50空格
        let tmp = std::env::temp_dir().join("aiw_regex_escape_patch_test.bin");
        let original: Vec<u8> = [
            &b"function isFp(){"[..],
            &b"if(Oe._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0"[..],
            &b";return!1}"[..],
        ]
        .concat();
        std::fs::write(&tmp, &original).unwrap();

        let pattern = UnifiedPatchPattern {
            feature: crate::patcher::types::FeatureType::AntiSpy,
            patch_type: PatchType::File,
            search_pattern: Cow::Borrowed(
                br"if\([a-zA-Z_$][a-zA-Z0-9_$]*\._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL\)return!0",
            ),
            replace_pattern: Some({
                let mut v = Vec::with_capacity(55);
                v.extend_from_slice(b"if(1)");
                v.extend(std::iter::repeat_n(b' ', 50));
                Cow::Owned(v)
            }),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test escape hatch"),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: None,
        };

        assert!(!is_file_patched(&tmp, &pattern).unwrap());
        let res = apply_file_patch(&tmp, &pattern);
        assert!(res.is_ok(), "{:?}", res);

        let patched = std::fs::read(&tmp).unwrap();
        assert_eq!(patched.len(), original.len(), "equal length 55->55");
        assert!(String::from_utf8_lossy(&patched).contains("if(1)"));
        assert!(!String::from_utf8_lossy(&patched).contains("if(Oe._CLAUDE_CODE"));
        assert!(is_file_patched(&tmp, &pattern).unwrap());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_apply_regex_dynamic_replace_replace_prefix() {
        // 模式 4 ReplacePrefix：if(FN())arr.push("**Provider context... 自适应 FN/arr 长度
        // 195 样本（FN=2B g7，arr=1B n）：span=63B，replace=if(0)+3空格+keep(58B)
        let tmp = std::env::temp_dir().join("aiw_regex_dyn_replace_prefix_test.bin");
        let original: Vec<u8> = [
            &b"...prefix..."[..],
            br#"if(g7())n.push("**Provider context:** This session is not using"#,
            &b"...suffix..."[..],
        ]
        .concat();
        std::fs::write(&tmp, &original).unwrap();

        let pattern = UnifiedPatchPattern {
            feature: crate::patcher::types::FeatureType::AntiPromptBias,
            patch_type: PatchType::File,
            // 组 1 = 后缀 `arr.push("**Provider context...`
            search_pattern: Cow::Borrowed(
                br#"if\([a-zA-Z_$][a-zA-Z0-9_$]*\(\)\)([a-zA-Z_$][a-zA-Z0-9_$]*\.push\("\*\*Provider context:\*\* This session is not using)"#,
            ),
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test dynamic replace ReplacePrefix"),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: Some(DynamicReplace::ReplacePrefix {
                keep_group: 1,
                prefix_literal: Cow::Borrowed(b"if(0)"),
            }),
        };

        // 未补丁：regex 命中 → false
        assert!(!is_file_patched(&tmp, &pattern).unwrap());
        let res = apply_file_patch(&tmp, &pattern);
        assert!(res.is_ok(), "{:?}", res);

        let patched = std::fs::read(&tmp).unwrap();
        // 等长
        assert_eq!(patched.len(), original.len());
        let s = String::from_utf8_lossy(&patched);
        // 条件 if(g7()) → if(0) 恒假（5B + 3 空格 = 8B 替换原 if(g7())=8B）
        assert!(s.contains("if(0)   n.push(\"**Provider context"));
        // push 语句原样保留（数组变量名 + prompt 不动）
        assert!(s.contains("n.push(\"**Provider context"));
        // 原 if(g7()) 已不存在
        assert!(!s.contains("if(g7())"));
        // 已补丁：regex 不再命中 → true
        assert!(is_file_patched(&tmp, &pattern).unwrap());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_apply_regex_dynamic_replace_replace_prefix_207() {
        // 207 样本（FN=3B yfe，arr=1B r）：span=64B（比 195 多 1B），空格自动 +1
        // 验证等长自动适应 FN 长度变化（根治点：固定 replace 63B 无法适配 64B）
        let tmp = std::env::temp_dir().join("aiw_regex_dyn_replace_prefix_207_test.bin");
        let original: Vec<u8> = [
            &b"...prefix..."[..],
            br#"if(yfe())r.push("**Provider context:** This session is not using"#,
            &b"...suffix..."[..],
        ]
        .concat();
        std::fs::write(&tmp, &original).unwrap();

        let pattern = UnifiedPatchPattern {
            feature: crate::patcher::types::FeatureType::AntiPromptBias,
            patch_type: PatchType::File,
            search_pattern: Cow::Borrowed(
                br#"if\([a-zA-Z_$][a-zA-Z0-9_$]*\(\)\)([a-zA-Z_$][a-zA-Z0-9_$]*\.push\("\*\*Provider context:\*\* This session is not using)"#,
            ),
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test dynamic replace ReplacePrefix 207"),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: Some(DynamicReplace::ReplacePrefix {
                keep_group: 1,
                prefix_literal: Cow::Borrowed(b"if(0)"),
            }),
        };

        assert!(!is_file_patched(&tmp, &pattern).unwrap());
        let res = apply_file_patch(&tmp, &pattern);
        assert!(res.is_ok(), "{:?}", res);

        let patched = std::fs::read(&tmp).unwrap();
        // 等长（64B → 64B，空格数 4 而非 195 的 3）
        assert_eq!(patched.len(), original.len());
        let s = String::from_utf8_lossy(&patched);
        // if(0) + 4 空格 = 9B 替换原 if(yfe())=9B
        assert!(s.contains("if(0)    r.push(\"**Provider context"));
        assert!(s.contains("r.push(\"**Provider context"));
        assert!(!s.contains("if(yfe())"));
        assert!(is_file_patched(&tmp, &pattern).unwrap());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_apply_regex_dynamic_replace_keep_prefix() {
        // 模式 4 KeepPrefix：function FN(){let e=BF()?.atis...void 0} → function FN(){return void 0 + 空格 + }
        // 验证函数名保留 + 函数体替换 + 等长自适应 FN 长度
        let tmp = std::env::temp_dir().join("aiw_regex_dyn_keep_prefix_test.bin");
        // 组 1 = `function R0i(){`（15B），函数体不含 }（闭合 } 单独拼接作为 regex 的 \} 锚点）
        let body = b"let e=mL()?.atis;if(e)return e;return void 0";
        let keep = b"function R0i(){";
        let end = b"}";
        // 拼成完整函数：keep + body + end
        let mut original: Vec<u8> = Vec::new();
        original.extend_from_slice(b"...prefix...");
        original.extend_from_slice(keep);
        original.extend_from_slice(body);
        original.extend_from_slice(end);
        original.extend_from_slice(b"...suffix...");
        let full_fn_len = keep.len() + body.len() + end.len();
        std::fs::write(&tmp, &original).unwrap();

        // suffix_literal=return void 0(13B)，end_literal=}（1B）
        // pad = full_fn_len - keep_len - suffix_len - end_len
        let expected_pad =
            full_fn_len - keep.len() - b"return void 0".len() - end.len();

        let pattern = UnifiedPatchPattern {
            feature: crate::patcher::types::FeatureType::AntiAtis,
            patch_type: PatchType::File,
            // 组 1 = `function R0i(){`
            search_pattern: Cow::Borrowed(
                br#"(function [a-zA-Z_$][a-zA-Z0-9_$]*\(\)\{)let e=[a-zA-Z_$][a-zA-Z0-9_$]*\(\)\?\.atis[\s\S]*?void 0\}"#,
            ),
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test dynamic replace KeepPrefix"),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: Some(DynamicReplace::KeepPrefix {
                keep_group: 1,
                suffix_literal: Cow::Borrowed(b"return void 0"),
                end_literal: Cow::Borrowed(b"}"),
            }),
        };

        assert!(!is_file_patched(&tmp, &pattern).unwrap());
        let res = apply_file_patch(&tmp, &pattern);
        assert!(res.is_ok(), "{:?}", res);

        let patched = std::fs::read(&tmp).unwrap();
        // 等长
        assert_eq!(patched.len(), original.len());
        let s = String::from_utf8_lossy(&patched);
        // 函数名保留
        assert!(s.contains("function R0i(){"));
        // 函数体替换为 return void 0 + 空格填充 + }
        let expected_body = format!(
            "function R0i(){{return void 0{}{}",
            " ".repeat(expected_pad),
            "}"
        );
        assert!(
            s.contains(&expected_body),
            "expected patched body {:?} in {:?}",
            expected_body,
            s.as_ref()
        );
        // 原 atis 提取逻辑已不存在
        assert!(!s.contains("?.atis"));
        assert!(is_file_patched(&tmp, &pattern).unwrap());

        let _ = std::fs::remove_file(&tmp);
    }
}
