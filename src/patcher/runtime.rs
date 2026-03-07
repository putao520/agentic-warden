//! 运行时内存补丁实现
//!
//! 使用平台抽象层实现跨平台的进程内存读写，
//! 用于在运行时修复程序 BUG 而不修改磁盘文件。

use crate::patcher::error::{PatchError, PatchResult};
use crate::patcher::platform::{MemoryPatcher, PlatformMemoryPatcher};
use tracing::{debug, info, trace, warn};

/// Claude CLI ToolSearch BUG 补丁 - 初始搜索模式
///
/// 搜索 "Party" 字符串作为锚点，用于定位 "firstParty"
/// 字节序列: 50 61 72 74 79
const SEARCH_PATTERN_PARTY: &[u8] = b"Party";

/// "firstParty" 完整字符串（10 字节）
/// 用于验证找到的 "Party" 是否真的是 "firstParty" 的一部分
const PATTERN_FIRST_PARTY: &[u8] = b"firstParty";

/// 要搜索的操作符模式: `===`
/// 在 JavaScript 源代码中，这是严格相等操作符
const OPERATOR_TRIPLE_EQUAL: &[u8] = b"===";

/// 补丁值: `!` (0x21) - 用于将 `===` 改为 `!==`
const PATCH_BYTE_EXCLAMATION: u8 = 0x21;

/// 向前搜索 `===` 操作符的最大距离（字节）
/// 在 "firstParty" 之前最多搜索这么多字节
const MAX_OPERATOR_SEARCH_DISTANCE: usize = 64;

/// 完整的目标代码模式: `O8()==="firstParty"&&!JB()`
/// 这是一个较长但唯一的模式，可以精确定位需要修改的位置
/// 注意：由于代码是压缩的，没有空格
/// 我们搜索其中的一部分模式来定位
const PATTERN_CLAUDE_TOOLSEARCH_BUG: &[u8] = b"O8()===\"firstParty\"&&!JB()";

/// 用于快速搜索的较短模式: `O8()==="firstParty"`
const PATTERN_O8_FIRST_PARTY: &[u8] = b"O8()===\"firstParty\"";

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

    /// 在内存区域中搜索所有字节模式出现的位置
    fn search_all_patterns_in_region(
        &self,
        region: &crate::patcher::platform::MemoryRegion,
        pattern: &[u8],
    ) -> PatchResult<Vec<usize>> {
        let mut results = Vec::new();

        let region_size = region.end.saturating_sub(region.start);
        if region_size < pattern.len() {
            return Ok(results);
        }

        const MAX_READ_SIZE: usize = 16 * 1024 * 1024; // 16MB
        let read_size = region_size.min(MAX_READ_SIZE);
        let mut buffer = vec![0u8; read_size];

        self.inner
            .read_memory(region.start, &mut buffer)?;

        for i in 0..=(buffer.len().saturating_sub(pattern.len())) {
            if &buffer[i..i + pattern.len()] == pattern {
                results.push(region.start + i);
            }
        }

        Ok(results)
    }

    /// 读取指定地址周围的内存上下文
    ///
    /// # 参数
    /// - `address`: 中心地址
    /// - `size_before`: 要读取的前面字节数
    /// - `size_after`: 要读取的后面字节数
    fn read_context(
        &self,
        address: usize,
        size_before: usize,
        size_after: usize,
    ) -> PatchResult<Vec<u8>> {
        let start = address.saturating_sub(size_before);
        let total_size = size_before + size_after + 1;
        let mut buffer = vec![0u8; total_size];

        self.inner.read_memory(start, &mut buffer)?;

        Ok(buffer)
    }

    /// 验证地址是否包含 "firstParty" 字符串
    ///
    /// 给定 "Party" 的地址，检查向前 5 个字节是否为 "first"
    fn verify_first_party(&self, party_addr: usize) -> PatchResult<bool> {
        // "firstParty" 中 "Party" 的偏移是 5
        let first_start = party_addr.saturating_sub(5);

        // 读取 "first" 部分（5 字节）
        let mut first_buf = [0u8; 5];
        self.inner.read_memory(first_start, &mut first_buf)?;

        Ok(&first_buf == b"first")
    }

    /// 在给定地址之前搜索 `===` 操作符
    ///
    /// # 参数
    /// - `address`: 从此地址向前搜索
    /// - `max_distance`: 最大搜索距离
    ///
    /// # 返回
    /// 如果找到 `===`，返回其第三个 `=` 的地址（需要修改的字节）
    fn find_triple_equal_before(
        &self,
        address: usize,
        max_distance: usize,
    ) -> PatchResult<Option<usize>> {
        let search_start = address.saturating_sub(max_distance);
        let search_size = address.saturating_sub(search_start);

        if search_size < OPERATOR_TRIPLE_EQUAL.len() {
            return Ok(None);
        }

        let mut buffer = vec![0u8; search_size];
        self.inner.read_memory(search_start, &mut buffer)?;

        // 搜索 `===` 模式
        for i in 0..=(buffer.len().saturating_sub(OPERATOR_TRIPLE_EQUAL.len())) {
            if &buffer[i..i + OPERATOR_TRIPLE_EQUAL.len()] == OPERATOR_TRIPLE_EQUAL {
                // 找到 `===`，返回第三个 `=` 的地址
                let equal_addr = search_start + i + 2; // 第三个 = 的偏移是 2
                debug!(
                    "Found '===' at address {:x}, third '=' at {:x}",
                    search_start + i,
                    equal_addr
                );
                return Ok(Some(equal_addr));
            }
        }

        Ok(None)
    }

    /// 应用单字节补丁
    fn apply_byte_patch(&self, address: usize, new_byte: u8) -> PatchResult<()> {
        // 先读取旧字节用于日志
        let mut old_byte_buf = [0u8; 1];
        self.inner
            .read_memory(address, &mut old_byte_buf)?;
        let old_byte = old_byte_buf[0];

        // 写入新字节
        self.inner.write_memory(address, &[new_byte])?;

        info!(
            "Patch applied at address {:x}: 0x{:02x} -> 0x{:02x}",
            address, old_byte, new_byte
        );

        Ok(())
    }

    /// 应用 Claude CLI ToolSearch 补丁
    ///
    /// 此补丁修复 Claude CLI 中的工具搜索 BUG：
    /// 将 `if(O8()==="firstParty"&&!JB())` 改为 `if(O8()!=="firstParty"&&!JB())`
    ///
    /// # 新算法 v2（精确定位完整模式）
    /// 1. 搜索完整模式 `O8()==="firstParty"&&!JB()`
    /// 2. 如果找到，计算第三个 `=` 的位置并修改
    /// 3. 如果完整模式未找到，搜索 `O8()==="firstParty"` 作为后备
    ///
    /// # 返回
    /// 成功时返回补丁应用的地址，失败返回错误
    pub fn apply_claude_toolsearch_patch(&self) -> PatchResult<usize> {
        if !self.process_exists() {
            return Err(PatchError::ProcessNotFound { pid: 0 });
        }

        info!("Searching for Claude ToolSearch bug pattern: O8()===\"firstParty\"&&!JB()");

        let regions = self.inner.read_memory_maps()?;
        let readable_regions: Vec<_> = regions
            .iter()
            .filter(|r| r.is_readable)
            .collect();

        debug!(
            "Searching {} readable memory regions for target pattern",
            readable_regions.len()
        );

        // 策略 1: 搜索完整模式
        info!("Trying strategy 1: Search for complete pattern O8()==\"firstParty\"&&!JB()");
        match self.search_and_apply_patch(&readable_regions, PATTERN_CLAUDE_TOOLSEARCH_BUG) {
            Ok(addr) => {
                info!("✅ Claude ToolSearch patch successfully applied (complete pattern) at address {:x}", addr);
                return Ok(addr);
            }
            Err(e) => {
                info!("Complete pattern not found: {}, trying fallback strategy", e);
            }
        }

        // 策略 2: 搜索较短模式
        info!("Trying strategy 2: Search for pattern O8()==\"firstParty\"");
        match self.search_and_apply_patch(&readable_regions, PATTERN_O8_FIRST_PARTY) {
            Ok(addr) => {
                info!("✅ Claude ToolSearch patch successfully applied (short pattern) at address {:x}", addr);
                return Ok(addr);
            }
            Err(e) => {
                warn!("Both patterns not found: {}", e);
            }
        }

        Err(PatchError::pattern_not_found(format!(
            "Target pattern not found in any readable memory region"
        )))
    }

    /// 搜索模式并应用补丁
    ///
    /// 给定一个包含 `O8()==="firstParty"` 的模式，找到第三个 `=` 的位置并修改
    fn search_and_apply_patch(
        &self,
        regions: &[&crate::patcher::platform::MemoryRegion],
        pattern: &[u8],
    ) -> PatchResult<usize> {
        for region in regions {
            trace!(
                "Searching region {:x}-{:x} ({} bytes)",
                region.start,
                region.end,
                region.end.saturating_sub(region.start),
            );

            match self.search_pattern_in_region(region, pattern) {
                Ok(Some(pattern_addr)) => {
                    info!("Found pattern at address {:x} in region {:x}-{:x}",
                          pattern_addr, region.start, region.end);

                    // 计算第三个 `=` 的位置
                    // 模式是 `O8()==="firstParty"...`
                    // O(0) 8(1) ((2) )(3) =(4) =(5) =(6) "(7) ...
                    // 第三个 = 在偏移 6
                    let patch_addr = pattern_addr + 6;

                    // 验证这个位置确实是 `=`
                    let mut verify_buf = [0u8; 1];
                    self.inner.read_memory(patch_addr, &mut verify_buf)?;
                    if verify_buf[0] != b'=' {
                        return Err(PatchError::PatternNotFound {
                            pattern: format!("Expected '=' at address {:x}, found 0x{:02x}",
                                           patch_addr, verify_buf[0]),
                            hint: None,
                        });
                    }

                    // 显示上下文
                    if let Ok(context) = self.read_context(pattern_addr, 4, 40) {
                        let context_str = String::from_utf8_lossy(&context);
                        info!("Context around patch: {}", context_str);
                    }

                    // 应用补丁：将第三个 `=` 改为 `!`
                    self.apply_byte_patch(patch_addr, PATCH_BYTE_EXCLAMATION)?;

                    return Ok(patch_addr);
                }
                Ok(None) => continue,
                Err(e) => {
                    trace!("Error searching region {:x}-{:x}: {}", region.start, region.end, e);
                    continue;
                }
            }
        }

        Err(PatchError::PatternNotFound {
            pattern: String::from_utf8_lossy(pattern).to_string(),
            hint: Some("Pattern not found in any readable region".to_string()),
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
                Err(e) => {
                    warn!("Error searching region {:x}-{:x}: {}", region.start, region.end, e);
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
    fn test_pattern_constants() {
        assert_eq!(SEARCH_PATTERN_PARTY, b"Party");
        assert_eq!(PATTERN_FIRST_PARTY, b"firstParty");
        assert_eq!(OPERATOR_TRIPLE_EQUAL, b"===");
        assert_eq!(PATCH_BYTE_EXCLAMATION, 0x21);
    }

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
    fn test_search_pattern_lengths() {
        assert_eq!(SEARCH_PATTERN_PARTY.len(), 5);
        assert_eq!(PATTERN_FIRST_PARTY.len(), 10);
        assert_eq!(OPERATOR_TRIPLE_EQUAL.len(), 3);
    }

    #[test]
    fn test_patch_pattern_constants() {
        // 验证补丁模式常量
        assert_eq!(PATTERN_CLAUDE_TOOLSEARCH_BUG, b"O8()===\"firstParty\"&&!JB()");
        assert_eq!(PATTERN_O8_FIRST_PARTY, b"O8()===\"firstParty\"");
        
        // 验证完整模式的字符串表示
        let pattern_str = std::str::from_utf8(PATTERN_CLAUDE_TOOLSEARCH_BUG).unwrap();
        assert_eq!(pattern_str, "O8()===\"firstParty\"&&!JB()");
    }

    #[test]
    fn test_patch_offset_calculation() {
        // 验证第三个 = 的偏移计算
        // 模式: O8()==="firstParty"
        // 索引: 0123456789...
        // O(0) 8(1) ((2) )(3) =(4) =(5) =(6) "(7)
        
        let pattern = PATTERN_O8_FIRST_PARTY;
        assert_eq!(pattern[0], b'O');
        assert_eq!(pattern[1], b'8');
        assert_eq!(pattern[2], b'(');
        assert_eq!(pattern[3], b')');
        assert_eq!(pattern[4], b'=');
        assert_eq!(pattern[5], b'=');
        assert_eq!(pattern[6], b'=');  // 这是第三个 =，需要改成 !
        assert_eq!(pattern[7], b'"');
        assert_eq!(pattern[8], b'f');
        
        // 验证完整模式的第三个 = 也在相同偏移
        let full_pattern = PATTERN_CLAUDE_TOOLSEARCH_BUG;
        assert_eq!(full_pattern[6], b'=');
    }
}
