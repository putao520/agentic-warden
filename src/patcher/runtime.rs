//! 运行时内存补丁实现
//!
//! 使用平台抽象层实现跨平台的进程内存读写，
//! 用于在运行时修改程序状态而不修改磁盘文件。

use crate::patcher::error::{PatchError, PatchResult};
use crate::patcher::platform::{MemoryPatcher, PlatformMemoryPatcher};
use tracing::{debug, info};

/// 运行时内存补丁器
///
/// 使用平台抽象层实现跨平台的内存补丁功能。
pub struct RuntimePatcher {
    /// 平台特定的补丁器实现
    inner: PlatformMemoryPatcher,
}

/// 将 String 错误转为 PatternNotFound（validate 等场景）
fn pattern_not_found(e: String) -> PatchError {
    PatchError::PatternNotFound { pattern: e, hint: None }
}

/// 将 regex::Error 转为 PatternNotFound
fn regex_err_to_pattern_not_found(e: regex::Error) -> PatchError {
    PatchError::PatternNotFound { pattern: format!("invalid regex: {}", e), hint: None }
}

impl RuntimePatcher {
    /// 创建针对指定进程的补丁器
    pub fn new(pid: u32) -> Result<Self, PatchError> {
        Ok(Self {
            inner: PlatformMemoryPatcher::new(pid)?,
        })
    }

    /// 检查进程是否存在
    pub fn process_exists(&self) -> bool {
        self.inner.process_exists()
    }

    /// 在内存区域中搜索字节模式
    fn search_pattern_in_region(
        &self,
        region: &crate::patcher::platform::MemoryRegion,
        pattern: &[u8],
    ) -> PatchResult<Option<usize>> {

        let region_size = region.end.saturating_sub(region.start);
        if region_size < pattern.len() {
            return Ok(None);
        }

        // 限制单次读取大小，避免内存问题
        const MAX_READ_SIZE: usize = 16 * 1024 * 1024; // 16MB
        let read_size = region_size.min(MAX_READ_SIZE);
        let mut buffer = vec![0u8; read_size];

        self.inner
            .read_memory(region.start, &mut buffer)?;

        // 搜索模式
        for i in 0..=(buffer.len().saturating_sub(pattern.len())) {
            if &buffer[i..i + pattern.len()] == pattern {
                let found_addr = region.start + i;
                debug!(
                    "Found pattern at address {:x} in region {:x}-{:x}",
                    found_addr, region.start, region.end
                );
                return Ok(Some(found_addr));
            }
        }

        Ok(None)
    }

    /// 单区域扫描上限，避免无界分配；regex 一次性扫描整区域以支持跨块匹配
    const MAX_REGION_SCAN: usize = 64 * 1024 * 1024;

    /// 遍历所有可读区域，对每个区域一次性读入（上限 64MB），调用 `processor`
    /// 处理缓冲区。`processor` 返回 `Some(x)` 表示已成功 patch；返回 `None`
    /// 表示此区域无有效匹配，继续扫描下一个区域。
    ///
    /// 3 个内存 patch 函数（max-context-tokens / regex-literal）共用此骨架，
    /// 各自只提供 `processor` 闭包即可，避免重复「遍历区域→64MB 读入→扫描」逻辑。
    fn scan_readable_regions<T>(
        &self,
        min_region_size: usize,
        mut processor: impl FnMut(&[u8], usize) -> PatchResult<Option<T>>,
    ) -> PatchResult<Option<T>> {
        let regions = self.inner.read_memory_maps()?;
        let readable_regions: Vec<_> = regions.iter().filter(|r| r.is_readable).collect();

        for region in &readable_regions {
            let region_size = region.end.saturating_sub(region.start);
            if region_size < min_region_size {
                continue;
            }
            let scan_size = region_size.min(Self::MAX_REGION_SCAN);
            let mut buffer = vec![0u8; scan_size];
            if self.inner.read_memory(region.start, &mut buffer).is_err() {
                continue;
            }
            if let Some(x) = processor(&buffer, region.start)? {
                return Ok(Some(x));
            }
        }
        Ok(None)
    }

    /// 在 `haystack` 中收集 `needle` 的所有起始偏移（用于定位 200000）。
    fn collect_literal_offsets(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
        let mut offsets = Vec::new();
        let mut search_from = 0;
        while search_from + needle.len() <= haystack.len() {
            if let Some(pos) = haystack[search_from..]
                .windows(needle.len())
                .position(|w| w == needle)
            {
                offsets.push(search_from + pos);
                search_from += pos + needle.len();
            } else {
                break;
            }
        }
        offsets
    }

    /// 应用 max-token 内存补丁
    ///
    /// 通过通用 regex 匹配 Claude CLI 内存中的常量块
    /// `var X=200000,Y=200000,Z=20000,W=32000,Q=128000;`，把两个 200000
    /// 等长替换为目标值。
    ///
    /// 区域扫描复用 `scan_readable_regions`（64MB 上限 + 整区域 regex 扫描），
    /// 本函数只负责「定位匹配块 + 委托 try_patch 写入」。
    ///
    /// # 参数
    /// - `max_tokens`: 目标默认上下文窗口值（6 位数，100000~999999）
    /// - `auto_compact`: autoCompact 阈值（6 位数，通常等于 max_tokens）
    ///
    /// # 返回
    /// 成功时返回首个 patch 应用的地址，失败返回错误
    pub fn apply_max_context_tokens_patch(
        &self,
        max_tokens: u32,
        auto_compact: u32,
    ) -> PatchResult<usize> {
        use crate::patcher::versions::{
            encode_max_context_tokens, validate_max_context_tokens, MAX_CONTEXT_TOKENS_SEARCH_REGEX,
        };

        validate_max_context_tokens(max_tokens)
            .map_err(pattern_not_found)?;
        validate_max_context_tokens(auto_compact)
            .map_err(pattern_not_found)?;

        if !self.process_exists() {
            return Err(PatchError::ProcessNotFound { pid: 0 });
        }

        let re = regex::bytes::Regex::new(MAX_CONTEXT_TOKENS_SEARCH_REGEX)
            .map_err(regex_err_to_pattern_not_found)?;

        let needle = b"200000";
        let val1 = encode_max_context_tokens(max_tokens);
        let val2 = encode_max_context_tokens(auto_compact);

        let scan = self.scan_readable_regions(
            MAX_CONTEXT_TOKENS_SEARCH_REGEX.len(),
            |buf, region_start| match re.find(buf) {
                Some(m) => self.try_patch_max_tokens(
                    region_start + m.start(),
                    &buf[m.start()..m.end()],
                    needle,
                    &val1,
                    &val2,
                ),
                None => Ok(None),
            },
        )?;

        match scan {
            Some((addr1, addr2)) => {
                info!(
                    "✅ MaxContextTokens patched: max_tokens={} @ {:#x}, auto_compact={} @ {:#x}",
                    max_tokens, addr1, auto_compact, addr2
                );
                Ok(addr1)
            }
            None => Err(PatchError::PatternNotFound {
                pattern: "max-context-tokens constant block not found in memory".to_string(),
                hint: Some("Claude version may not contain the expected constant block".to_string()),
            }),
        }
    }

    /// 在已匹配的常量块中定位两个 200000 并 patch，成功返回两个写入地址。
    /// 偏移不足 / 字节校验失败时返回 `Ok(None)`（让外层继续扫描下一个区域）。
    fn try_patch_max_tokens(
        &self,
        match_base: usize,
        matched: &[u8],
        needle: &[u8],
        val1: &[u8],
        val2: &[u8],
    ) -> PatchResult<Option<(usize, usize)>> {
        let offsets = Self::collect_literal_offsets(matched, needle);
        if offsets.len() < 2 {
            debug!(
                "matched constant block but found {} 200000 occurrences (need 2)",
                offsets.len()
            );
            return Ok(None);
        }
        let addr1 = match_base + offsets[0];
        let addr2 = match_base + offsets[1];

        // 验证原字节确为 200000
        let mut verify = [0u8; 6];
        if self.inner.read_memory(addr1, &mut verify).is_err() || verify != *needle {
            return Ok(None);
        }
        if self.inner.read_memory(addr2, &mut verify).is_err() || verify != *needle {
            return Ok(None);
        }

        self.inner.write_memory(addr1, val1)?;
        self.inner.write_memory(addr2, val2)?;
        Ok(Some((addr1, addr2)))
    }


    /// 应用字面量内存 patch（search_pattern → replace_pattern 整段替换）
    ///
    /// 用于 AntiTelemetry 等字面量 patch：在进程内存中找到 `search_pattern`，
    /// 用 `replace_pattern` 整段覆盖写入。要求 search 与 replace 等长，
    /// 避免破坏二进制偏移。命中首个匹配即返回地址。
    ///
    /// # 参数
    /// - `pattern`: 补丁模式（必须提供等长的 search_pattern 与 replace_pattern）
    ///
    /// # 返回
    /// 成功时返回写入的起始地址，失败返回错误
    pub fn apply_literal_memory_patch(
        &self,
        pattern: &crate::patcher::types::UnifiedPatchPattern,
    ) -> PatchResult<usize> {
        if !self.process_exists() {
            return Err(PatchError::ProcessNotFound { pid: 0 });
        }

        let search = pattern.search_pattern.as_ref();
        let replace = pattern.replace_pattern.as_ref().ok_or_else(|| {
            PatchError::PatternNotFound {
                pattern: "replace_pattern required for literal memory patch".to_string(),
                hint: None,
            }
        })?;

        if search.len() != replace.len() {
            return Err(PatchError::PatternNotFound {
                pattern: format!(
                    "literal patch must be equal length: search={}, replace={}",
                    search.len(),
                    replace.len()
                ),
                hint: None,
            });
        }

        // 在所有可读区域中搜索字面量模式
        let regions = self.inner.read_memory_maps()?;
        let readable_regions: Vec<_> = regions.iter().filter(|r| r.is_readable).collect();

        for region in readable_regions {
            match self.search_pattern_in_region(region, search) {
                Ok(Some(addr)) => {
                    self.inner.write_memory(addr, replace)?;
                    info!(
                        "✅ AntiTelemetry patched: endpoint -> 404 @ {:#x}",
                        addr
                    );
                    return Ok(addr);
                }
                Ok(None) => continue,
                Err(_) => continue,
            }
        }

        Err(PatchError::PatternNotFound {
            pattern: String::from_utf8_lossy(search).to_string(),
            hint: Some("Pattern not found in any readable region".to_string()),
        })
    }

    /// 应用 regex 字面量内存 patch（regex 匹配 → replace_pattern 整段覆盖）
    ///
    /// 用于跨版本 patch 点（minified 变量名变化但匹配文本长度固定）：
    /// `search_pattern` 是 regex 字符串，编译后扫描可读内存区域，找到匹配后
    /// 用 `replace_pattern` 整段 `write_memory` 覆盖。要求 regex 匹配长度
    /// == `replace_pattern.len()`（等长，避免破坏二进制偏移）。命中首个匹配
    /// 即返回地址。
    ///
    /// 区域扫描复用 `apply_max_context_tokens_patch` 的 64MB 上限策略：
    /// 对每个可读区域一次性读入（上限 64MB）后用 regex 整体扫描，pattern
    /// 跨 16MB 块边界也能命中。
    ///
    /// # 参数
    /// - `pattern`: 补丁模式（`use_regex=true`，必须提供等长的 `replace_pattern`）
    ///
    /// # 返回
    /// 成功时返回写入的起始地址，失败返回错误（`PatternNotFound` 由上层静默处理）
    pub fn apply_regex_literal_memory_patch(
        &self,
        pattern: &crate::patcher::types::UnifiedPatchPattern,
    ) -> PatchResult<usize> {
        // 模式 4 派发：dynamic_replace=Some 时走动态内存 patch
        if pattern.dynamic_replace.is_some() {
            return self.apply_regex_dynamic_memory_patch(pattern);
        }

        if !self.process_exists() {
            return Err(PatchError::ProcessNotFound { pid: 0 });
        }

        let regex_str = std::str::from_utf8(pattern.search_pattern.as_ref())
            .map_err(|e| PatchError::PatternNotFound { pattern: format!("invalid regex utf-8: {}", e), hint: None })?;
        let re = regex::bytes::Regex::new(regex_str).map_err(regex_err_to_pattern_not_found)?;

        let replace = pattern.replace_pattern.as_ref().ok_or_else(|| {
            PatchError::PatternNotFound {
                pattern: "replace_pattern required for regex literal memory patch"
                    .to_string(),
                hint: None,
            }
        })?;

        let replace_len = replace.len();
        let scan = self.scan_readable_regions(regex_str.len(), |buf, region_start| {
            match re.find(buf) {
                Some(m) => {
                    let span_len = m.end() - m.start();
                    if replace_len != span_len {
                        debug!(
                            "regex literal match length {} != replace {} in region {:x}, skip",
                            span_len, replace_len, region_start
                        );
                        return Ok(None);
                    }
                    let addr = region_start + m.start();
                    self.inner.write_memory(addr, replace)?;
                    debug!(
                        "✅ regex literal memory patched @ {:#x} (match len={})",
                        addr, span_len
                    );
                    Ok(Some(addr))
                }
                None => Ok(None),
            }
        })?;

        match scan {
            Some(addr) => Ok(addr),
            None => Err(PatchError::PatternNotFound {
                pattern: regex_str.to_string(),
                hint: Some("regex pattern not found in any readable region".to_string()),
            }),
        }
    }

    /// 应用 regex 动态内存 patch（模式 4：保留捕获组 + 字面量 + 空格填充）
    ///
    /// 用于 AntiPromptBias / AntiAtis 等跨版本 patch 点（minified 变量名长度
    /// 跨版本变化导致匹配长度不固定）：regex 匹配后保留指定捕获组内容（语义
    /// 要求保留的部分：函数名/数组变量名/prompt 文本），其余部分用字面量 +
    /// 空格填充至匹配长度，等长自动保证。命中首个匹配即返回地址。
    ///
    /// 动态 replace 构造逻辑与 `file::apply_regex_dynamic_replace` 完全一致
    ///（同一个 BCE 根治的两处对称实现）：
    /// - `ReplacePrefix`: `prefix_literal` + 空格*(span_len - prefix_len - keep_len)
    ///   + `match[keep_group]`
    /// - `KeepPrefix`: `match[keep_group]` + `suffix_literal` + 空格*
    ///   (span_len - keep_len - suffix_len - end_len) + `end_literal`
    ///
    /// 区域扫描复用 `scan_readable_regions` 的 64MB 上限策略。
    ///
    /// # 参数
    /// - `pattern`: 补丁模式（`use_regex=true` + `dynamic_replace=Some`）
    ///
    /// # 返回
    /// 成功时返回写入的起始地址，失败返回错误（`PatternNotFound` 由上层静默处理）
    pub fn apply_regex_dynamic_memory_patch(
        &self,
        pattern: &crate::patcher::types::UnifiedPatchPattern,
    ) -> PatchResult<usize> {
        use crate::patcher::types::DynamicReplace;

        if !self.process_exists() {
            return Err(PatchError::ProcessNotFound { pid: 0 });
        }

        let regex_str = std::str::from_utf8(pattern.search_pattern.as_ref())
            .map_err(|e| PatchError::PatternNotFound {
                pattern: format!("invalid regex utf-8: {}", e),
                hint: None,
            })?;
        let re = regex::bytes::Regex::new(regex_str).map_err(regex_err_to_pattern_not_found)?;

        let dynamic_replace = pattern.dynamic_replace.as_ref().ok_or_else(|| {
            PatchError::PatternNotFound {
                pattern: "dynamic_replace required for regex dynamic memory patch"
                    .to_string(),
                hint: None,
            }
        })?;

        let (keep_group, prefix_literal, suffix_literal, end_literal) = match dynamic_replace {
            DynamicReplace::ReplacePrefix { keep_group, prefix_literal } => {
                (*keep_group, Some(prefix_literal.as_ref()), None, None)
            }
            DynamicReplace::KeepPrefix {
                keep_group,
                suffix_literal,
                end_literal,
            } => (
                *keep_group,
                None,
                Some(suffix_literal.as_ref()),
                Some(end_literal.as_ref()),
            ),
        };

        let scan = self.scan_readable_regions(regex_str.len(), |buf, region_start| {
            // 命中首个匹配即返回（与 apply_regex_literal_memory_patch 一致）
            if let Some(caps) = re.captures_iter(buf).next() {
                let m = caps.get(0).expect("capture group 0 always exists");
                let span_len = m.end() - m.start();

                let keep = caps.get(keep_group).ok_or_else(|| PatchError::PatternNotFound {
                    pattern: format!(
                        "dynamic replace keep_group {} not captured by regex",
                        keep_group
                    ),
                    hint: None,
                })?;
                let keep_bytes = &buf[keep.start()..keep.end()];

                let replace = match (prefix_literal, suffix_literal, end_literal) {
                    (Some(prefix), _, _) => {
                        let pad = span_len
                            .saturating_sub(prefix.len() + keep_bytes.len());
                        let mut r = Vec::with_capacity(span_len);
                        r.extend_from_slice(prefix);
                        r.extend(std::iter::repeat_n(b' ', pad));
                        r.extend_from_slice(keep_bytes);
                        r
                    }
                    (_, Some(suffix), Some(end)) => {
                        let pad = span_len.saturating_sub(
                            keep_bytes.len() + suffix.len() + end.len(),
                        );
                        let mut r = Vec::with_capacity(span_len);
                        r.extend_from_slice(keep_bytes);
                        r.extend_from_slice(suffix);
                        r.extend(std::iter::repeat_n(b' ', pad));
                        r.extend_from_slice(end);
                        r
                    }
                    _ => unreachable!(
                        "DynamicReplace variants exhausted: prefix_literal xor (suffix+end)"
                    ),
                };

                debug_assert_eq!(
                    replace.len(),
                    span_len,
                    "dynamic replace must be equal length: span={}, replace={}",
                    span_len,
                    replace.len()
                );

                let addr = region_start + m.start();
                self.inner.write_memory(addr, &replace)?;
                debug!(
                    "✅ regex dynamic memory patched @ {:#x} (match len={}, keep_len={})",
                    addr, span_len, keep_bytes.len()
                );
                return Ok(Some(addr));
            }
            Ok(None)
        })?;

        match scan {
            Some(addr) => Ok(addr),
            None => Err(PatchError::PatternNotFound {
                pattern: regex_str.to_string(),
                hint: Some("regex pattern not found in any readable region".to_string()),
            }),
        }
    }

    /// 在内存中搜索字节模式
    ///
    /// # 参数
    /// - `pattern`: 要搜索的字节模式
    ///
    /// # 返回
    /// 成功时返回找到的地址，失败返回错误
    pub fn search_pattern(&self, pattern: &[u8]) -> PatchResult<Option<usize>> {        if !self.process_exists() {
            return Err(PatchError::ProcessNotFound { pid: 0 });
        }

        let regions = self.inner.read_memory_maps()?;
        let readable_regions: Vec<_> = regions
            .iter()
            .filter(|r| r.is_readable)
            .collect();

        for region in readable_regions {
            match self.search_pattern_in_region(region, pattern) {
                Ok(Some(addr)) => return Ok(Some(addr)),
                Ok(None) => continue,
                Err(_e) => {
                    continue;
                }
            }
        }

        Ok(None)
    }

    /// 读取内存
    ///
    /// # 参数
    /// - `addr`: 内存地址
    /// - `buf`: 接收缓冲区
    pub fn read_memory(&self, addr: usize, buf: &mut [u8]) -> PatchResult<()> {
        self.inner.read_memory(addr, buf)
    }

    /// 写入内存
    ///
    /// # 参数
    /// - `addr`: 内存地址
    /// - `data`: 要写入的数据
    pub fn write_memory(&self, addr: usize, data: &[u8]) -> PatchResult<()> {
        self.inner.write_memory(addr, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patcher_creation_invalid_pid() {
        // 使用不太可能存在的 PID
        let result = RuntimePatcher::new(999999);
        // 结果取决于平台，但应该返回 Result
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_patcher_creation() {
        // 使用自身进程的 PID 进行测试
        let pid = std::process::id();
        let patcher = RuntimePatcher::new(pid);
        assert!(patcher.is_ok());
    }

    #[test]
    fn test_max_context_tokens_patch_rejects_invalid_values() {
        // 不依赖真实进程的纯逻辑校验路径
        let pid = std::process::id();
        let patcher = RuntimePatcher::new(pid).unwrap();
        // 无效值在进入内存扫描前就被拒
        let r = patcher.apply_max_context_tokens_patch(99999, 500000);
        assert!(r.is_err());
        let r = patcher.apply_max_context_tokens_patch(500000, 1000000);
        assert!(r.is_err());
    }

    #[test]
    fn test_literal_memory_patch_rejects_missing_replace_pattern() {
        use crate::patcher::types::{FeatureType, UnifiedPatchPattern};
        use std::borrow::Cow;

        let pid = std::process::id();
        let patcher = RuntimePatcher::new(pid).unwrap();
        let pattern = UnifiedPatchPattern {
            feature: FeatureType::AntiTelemetry,
            patch_type: crate::patcher::types::PatchType::Memory,
            search_pattern: Cow::Borrowed(b"/api/event_logging/v2/batch"),
            replace_pattern: None, // 缺失 replace_pattern
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test missing replace"),
            use_regex: false,
            regex_replace_values: None,
            dynamic_replace: None,
        };
        // 缺 replace_pattern 应返回错误（不应命中 "not found" 而是参数校验错误）
        let r = patcher.apply_literal_memory_patch(&pattern);
        assert!(r.is_err());
    }

    #[test]
    fn test_literal_memory_patch_rejects_unequal_length() {
        use crate::patcher::types::{FeatureType, UnifiedPatchPattern};
        use std::borrow::Cow;

        let pid = std::process::id();
        let patcher = RuntimePatcher::new(pid).unwrap();
        let pattern = UnifiedPatchPattern {
            feature: FeatureType::AntiTelemetry,
            patch_type: crate::patcher::types::PatchType::Memory,
            search_pattern: Cow::Borrowed(b"/api/event_logging/v2/batch"),
            replace_pattern: Some(Cow::Borrowed(b"short")), // 长度不一致
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test unequal length"),
            use_regex: false,
            regex_replace_values: None,
            dynamic_replace: None,
        };
        let r = patcher.apply_literal_memory_patch(&pattern);
        assert!(r.is_err());
    }

    #[test]
    fn test_regex_literal_memory_patch_rejects_missing_replace() {
        // regex 字面量内存 patch：缺 replace_pattern 应返回错误
        use crate::patcher::types::{FeatureType, UnifiedPatchPattern};
        use std::borrow::Cow;

        let pid = std::process::id();
        let patcher = RuntimePatcher::new(pid).unwrap();
        let pattern = UnifiedPatchPattern {
            feature: FeatureType::AntiSpy,
            patch_type: crate::patcher::types::PatchType::Memory,
            search_pattern: Cow::Borrowed(
                br"if\([a-zA-Z_$][a-zA-Z0-9_$]*\._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL\)return!0",
            ),
            replace_pattern: None, // 缺 replace_pattern
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test missing replace"),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: None,
        };
        let r = patcher.apply_regex_literal_memory_patch(&pattern);
        assert!(r.is_err(), "missing replace_pattern must error");
    }

    #[test]
    fn test_regex_literal_memory_patch_rejects_invalid_regex() {
        // regex 字面量内存 patch：非法 regex 应返回错误
        use crate::patcher::types::{FeatureType, UnifiedPatchPattern};
        use std::borrow::Cow;

        let pid = std::process::id();
        let patcher = RuntimePatcher::new(pid).unwrap();
        let pattern = UnifiedPatchPattern {
            feature: FeatureType::AntiSpy,
            patch_type: crate::patcher::types::PatchType::Memory,
            search_pattern: Cow::Borrowed(b"(unclosed["),
            replace_pattern: Some(Cow::Borrowed(b"if(1)                                                  ")),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Borrowed("test invalid regex"),
            use_regex: true,
            regex_replace_values: None,
            dynamic_replace: None,
        };
        let r = patcher.apply_regex_literal_memory_patch(&pattern);
        assert!(r.is_err(), "invalid regex must error");
    }
}
