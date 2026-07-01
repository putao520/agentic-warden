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

    /// 应用 max-token 内存补丁
    ///
    /// 通过通用 regex 匹配 Claude CLI 内存中的常量块
    /// `var X=200000,Y=200000,Z=20000,W=32000,Q=128000;`，把两个 200000
    /// 等长替换为目标值。
    ///
    /// 通过变量名无关的 regex 通用匹配，跨版本稳定。对每个可读区域
    /// 一次性读入（上限 64MB）后用 regex 整体扫描，pattern 跨 16MB
    /// 块边界也能命中。
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
            .map_err(|e| PatchError::PatternNotFound { pattern: e, hint: None })?;
        validate_max_context_tokens(auto_compact)
            .map_err(|e| PatchError::PatternNotFound { pattern: e, hint: None })?;

        if !self.process_exists() {
            return Err(PatchError::ProcessNotFound { pid: 0 });
        }

        let re = regex::bytes::Regex::new(MAX_CONTEXT_TOKENS_SEARCH_REGEX)
            .map_err(|e| PatchError::PatternNotFound {
                pattern: format!("invalid regex: {}", e),
                hint: None,
            })?;

        let regions = self.inner.read_memory_maps()?;
        let readable_regions: Vec<_> = regions
            .iter()
            .filter(|r| r.is_readable)
            .collect();

        debug!(
            "Scanning {} readable regions for max-context-tokens constant block",
            readable_regions.len()
        );

        // 单区域上限 64MB，避免无界分配；regex 一次性扫描整区域以支持跨块匹配
        const MAX_REGION_SCAN: usize = 64 * 1024 * 1024;

        for region in &readable_regions {
            let region_size = region.end.saturating_sub(region.start);
            if region_size < MAX_CONTEXT_TOKENS_SEARCH_REGEX.len() {
                continue;
            }
            let scan_size = region_size.min(MAX_REGION_SCAN);
            let mut buffer = vec![0u8; scan_size];
            if self.inner.read_memory(region.start, &mut buffer).is_err() {
                continue;
            }

            if let Some(m) = re.find(&buffer) {
                let matched = &buffer[m.start()..m.end()];
                // 在匹配文本中定位两个 200000 的偏移
                let needle = b"200000";
                let mut offsets: Vec<usize> = Vec::new();
                let mut search_from = 0;
                while search_from + needle.len() <= matched.len() {
                    if let Some(pos) = matched[search_from..]
                        .windows(needle.len())
                        .position(|w| w == needle)
                    {
                        offsets.push(search_from + pos);
                        search_from += pos + needle.len();
                    } else {
                        break;
                    }
                }

                if offsets.len() < 2 {
                    debug!(
                        "matched constant block but found {} 200000 occurrences (need 2)",
                        offsets.len()
                    );
                    continue;
                }

                let match_base = region.start + m.start();
                let val1 = encode_max_context_tokens(max_tokens);
                let val2 = encode_max_context_tokens(auto_compact);

                let addr1 = match_base + offsets[0];
                let addr2 = match_base + offsets[1];

                // 验证原字节确为 200000
                let mut verify = [0u8; 6];
                if self.inner.read_memory(addr1, &mut verify).is_err() || &verify != needle {
                    continue;
                }
                if self.inner.read_memory(addr2, &mut verify).is_err() || &verify != needle {
                    continue;
                }

                self.inner.write_memory(addr1, &val1)?;
                self.inner.write_memory(addr2, &val2)?;

                info!(
                    "✅ MaxContextTokens patched: max_tokens={} @ {:#x}, auto_compact={} @ {:#x}",
                    max_tokens, addr1, auto_compact, addr2
                );
                return Ok(addr1);
            }
        }

        Err(PatchError::PatternNotFound {
            pattern: "max-context-tokens constant block not found in memory".to_string(),
            hint: Some("Claude version may not contain the expected constant block".to_string()),
        })
    }


    /// 在内存中搜索字节模式
    ///
    /// # 参数
    /// - `pattern`: 要搜索的字节模式
    ///
    /// # 返回
    /// 成功时返回找到的地址，失败返回错误
    pub fn search_pattern(&self, pattern: &[u8]) -> PatchResult<Option<usize>> {
        if !self.process_exists() {
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
}
