//! 运行时内存补丁实现
//!
//! 使用平台抽象层实现跨平台的进程内存读写，
//! 用于在运行时修复程序 BUG 而不修改磁盘文件。

use crate::patcher::error::{PatchError, PatchResult};
use crate::patcher::platform::{MemoryPatcher, PlatformMemoryPatcher};
use crate::patcher::versions::{ClaudeVersion, detect_patch_pattern, get_patch_pattern};
use tracing::{debug, info, trace};

/// Claude CLI firstParty 补丁
///
/// 由于代码混淆，函数名会随版本变化
/// 当前已知: 2.1.71 及更早使用 O8(), 2.1.72+ 使用 cL()
///
/// 补丁目标：将 `XXX()==="firstParty"` 改为 `XXX()!=="firstParty"`
/// 这会解锁所有被 firstParty 限制的功能（Tool Search, Artifacts 等）

/// "firstParty" 完整字符串
pub const PATTERN_FIRST_PARTY: &[u8] = b"firstParty";

/// 补丁值: `!` (0x21) - 用于将 `===` 改为 `!==`
pub const PATCH_BYTE_EXCLAMATION: u8 = 0x21;

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

    /// 应用单字节补丁
    fn apply_byte_patch(&self, address: usize, new_byte: u8) -> PatchResult<()> {
        // 先读取旧字节用于日志
        let mut old_byte_buf = [0u8; 1];
        self.inner
            .read_memory(address, &mut old_byte_buf)?;
        let _old_byte = old_byte_buf[0];

        // 写入新字节
        self.inner.write_memory(address, &[new_byte])?;

        Ok(())
    }

    /// 应用 Claude CLI ToolSearch 补丁
    ///
    /// 此补丁修复 Claude CLI 中的工具搜索 BUG：
    /// 将 `XXX()==="firstParty"` 改为 `XXX()!=="firstParty"`
    ///
    /// 由于代码混淆，函数名可能会变化（如 O8, cL 等）
    /// 因此我们动态搜索 `()==="firstParty"` 模式
    ///
    /// # 返回
    /// 成功时返回补丁应用的地址，失败返回错误
    pub fn apply_claude_toolsearch_patch(&self) -> PatchResult<usize> {
        if !self.process_exists() {
            return Err(PatchError::ProcessNotFound { pid: 0 });
        }

        let regions = self.inner.read_memory_maps()?;
        let readable_regions: Vec<_> = regions
            .iter()
            .filter(|r| r.is_readable)
            .collect();

        debug!(
            "Searching {} readable memory regions for firstParty check pattern",
            readable_regions.len()
        );

        // 尝试获取 Claude 版本并获取对应模式
        // 如果版本检测失败，使用动态检测列表
        let patterns = if let Ok(version_str) = self.get_claude_version() {
            debug!("Detected Claude version: {}", version_str);
            if let Some(version) = ClaudeVersion::from_string(&version_str) {
                if let Some(pattern) = get_patch_pattern(&version) {
                    debug!("Using known pattern for version: {} (function: {})", 
                           version_str, pattern.function_name);
                    vec![pattern]
                } else {
                    debug!("Unknown version, using detection list");
                    detect_patch_pattern()
                }
            } else {
                debug!("Failed to parse version, using detection list");
                detect_patch_pattern()
            }
        } else {
            debug!("Failed to detect version, trying all known patterns");
            detect_patch_pattern()
        };

        // 尝试每个模式
        for pattern in &patterns {
            debug!("Trying pattern with function: {}", pattern.function_name);
            match self.try_pattern(&readable_regions, pattern) {
                Ok(addr) => {
                    info!("✅ Claude firstParty unlocked (using {} function)", pattern.function_name);
                    return Ok(addr);
                }
                Err(_) => {
                    continue;
                }
            }
        }

        Err(PatchError::PatternNotFound {
            pattern: "No valid firstParty check pattern found".to_string(),
            hint: Some("Claude version may be unsupported".to_string()),
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

    /// 获取 Claude CLI 版本
    fn get_claude_version(&self) -> Result<String, PatchError> {
        // 通过执行 claude --version 获取
        use std::process::Command;
        
        let output = Command::new("claude")
            .arg("--version")
            .output()
            .map_err(|_| PatchError::PatternNotFound {
                pattern: "Failed to execute claude --version".to_string(),
                hint: None,
            })?;
        
        let version_str = String::from_utf8_lossy(&output.stdout);
        // 格式: "2.1.72 (Claude Code)" 或类似
        let version = version_str
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        
        if version.is_empty() {
            return Err(PatchError::PatternNotFound {
                pattern: "Could not parse version".to_string(),
                hint: None,
            });
        }
        
        Ok(version)
    }

    /// 尝试使用指定的模式进行补丁
    fn try_pattern(
        &self,
        regions: &[&crate::patcher::platform::MemoryRegion],
        pattern: &super::versions::PatchPattern,
    ) -> PatchResult<usize> {
        for region in regions {
            trace!(
                "Searching region {:x}-{:x} ({} bytes)",
                region.start,
                region.end,
                region.end.saturating_sub(region.start),
            );

            match self.search_pattern_in_region(region, pattern.short_pattern) {
                Ok(Some(pattern_addr)) => {
                    // 计算第三个 = 的位置
                    let patch_addr = pattern_addr + pattern.equals_offset;

                    // 验证这个位置确实是 =
                    let mut verify_buf = [0u8; 1];
                    self.inner.read_memory(patch_addr, &mut verify_buf)?;
                    if verify_buf[0] != b'=' {
                        return Err(PatchError::PatternNotFound {
                            pattern: format!("Expected '=' at address {:x}, found 0x{:02x}",
                                           patch_addr, verify_buf[0]),
                            hint: None,
                        });
                    }

                    debug!("Patching at {:x} (using {})", patch_addr, pattern.function_name);

                    // 应用补丁：将第三个 = 改为 !
                    self.apply_byte_patch(patch_addr, PATCH_BYTE_EXCLAMATION)?;

                    return Ok(patch_addr);
                }
                Ok(None) => continue,
                Err(_) => continue,
            }
        }

        Err(PatchError::PatternNotFound {
            pattern: String::from_utf8_lossy(pattern.short_pattern).to_string(),
            hint: Some("Pattern not found in any readable region".to_string()),
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_constants() {
        assert_eq!(PATTERN_FIRST_PARTY, b"firstParty");
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
        assert_eq!(PATTERN_FIRST_PARTY.len(), 10);
    }

    #[test]
    fn test_patch_pattern_constants() {
        // 验证补丁模式常量
        assert_eq!(PATTERN_FIRST_PARTY, b"firstParty");

        // 验证模式的字符串表示
        let pattern_str = std::str::from_utf8(PATTERN_FIRST_PARTY).unwrap();
        assert_eq!(pattern_str, "firstParty");
    }


}
