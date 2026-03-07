//! Unix 平台内存补丁实现
//!
//! 支持 Linux（使用 /proc/pid/mem 和 /proc/pid/maps）

use crate::patcher::error::{PatchError, PatchResult};
use crate::patcher::platform::{MemoryPatcher, MemoryRegion, MemPerm};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use tracing::trace;

/// Unix 平台内存补丁器（Linux 专用）
pub struct PlatformMemoryPatcher {
    pid: u32,
    mem_file: Option<File>,
}

impl PlatformMemoryPatcher {
    /// 获取 /proc/pid/mem 路径
    fn mem_path(&self) -> String {
        format!("/proc/{}/mem", self.pid)
    }

    /// 获取 /proc/pid/maps 路径
    fn maps_path(&self) -> String {
        format!("/proc/{}/maps", self.pid)
    }

    /// 检查进程是否存在
    fn check_process_exists(&self) -> bool {
        Path::new(&format!("/proc/{}", self.pid)).exists()
    }

    /// 读取内存映射
    fn read_maps_linux(&self) -> PatchResult<Vec<MemoryRegion>> {
        let maps_path = self.maps_path();
        let content = std::fs::read_to_string(&maps_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PatchError::ProcessNotFound { pid: self.pid }
            } else {
                PatchError::ReadFailed {
                    reason: format!("Failed to read {}: {}", maps_path, e),
                }
            }
        })?;

        let mut regions = Vec::new();
        for line in content.lines() {
            if let Some(region) = Self::parse_maps_line(line) {
                regions.push(region);
            }
        }

        Ok(regions)
    }

    /// 解析 /proc/pid/maps 行
    fn parse_maps_line(line: &str) -> Option<MemoryRegion> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let addr_range: Vec<&str> = parts[0].split('-').collect();
        if addr_range.len() != 2 {
            return None;
        }

        let start = usize::from_str_radix(addr_range[0], 16).ok()?;
        let end = usize::from_str_radix(addr_range[1], 16).ok()?;
        let perms = MemPerm::from_maps_str(parts[1])?;

        let is_readable = perms.read;
        let is_writable = perms.write;

        Some(MemoryRegion {
            start,
            end,
            perms,
            is_readable,
            is_writable,
        })
    }

    /// 打开内存文件
    fn open_mem_file(&self) -> PatchResult<File> {
        let mem_path = self.mem_path();
        File::options()
            .read(true)
            .write(true)
            .open(&mem_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    PatchError::PermissionDenied {
                        reason: format!(
                            "Cannot access {}: try running with elevated privileges",
                            mem_path
                        ),
                    }
                } else {
                    PatchError::ReadFailed {
                        reason: format!("Failed to open {}: {}", mem_path, e),
                    }
                }
            })
    }
}

impl MemoryPatcher for PlatformMemoryPatcher {
    fn new(pid: u32) -> Result<Self, PatchError> {
        Ok(Self {
            pid,
            mem_file: None,
        })
    }

    fn process_exists(&self) -> bool {
        self.check_process_exists()
    }

    fn read_memory_maps(&self) -> PatchResult<Vec<MemoryRegion>> {
        self.read_maps_linux()
    }

    fn read_memory(&self, addr: usize, buf: &mut [u8]) -> PatchResult<()> {
        let mut mem_file = self.open_mem_file()?;
        mem_file
            .seek(SeekFrom::Start(addr as u64))
            .map_err(|e| PatchError::ReadFailed {
                reason: format!("Failed to seek to address {:x}: {}", addr, e),
            })?;

        mem_file.read_exact(buf).map_err(|e| PatchError::ReadFailed {
            reason: format!("Failed to read memory at {:x}: {}", addr, e),
        })?;

        trace!("Read {} bytes from address {:x}", buf.len(), addr);
        Ok(())
    }

    fn write_memory(&self, addr: usize, data: &[u8]) -> PatchResult<()> {
        let mut mem_file = self.open_mem_file()?;
        mem_file
            .seek(SeekFrom::Start(addr as u64))
            .map_err(|e| PatchError::WriteFailed {
                reason: format!("Failed to seek to address {:x}: {}", addr, e),
            })?;

        mem_file.write_all(data).map_err(|e| PatchError::WriteFailed {
            reason: format!("Failed to write memory at {:x}: {}", addr, e),
        })?;

        tracing::debug!("Wrote {} bytes to address {:x}", data.len(), addr);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_maps_line() {
        let line = "7f8a1b000000-7f8a1b001000 rw-p 00000000 00:00 0          [heap]";
        let region = PlatformMemoryPatcher::parse_maps_line(line).unwrap();
        assert_eq!(region.start, 0x7f8a1b000000);
        assert_eq!(region.end, 0x7f8a1b001000);
        assert!(region.is_readable);
        assert!(region.is_writable);
    }

    #[test]
    fn test_parse_maps_line_text_segment() {
        let line = "400000-410000 r-xp 00000000 08:01 12345 /usr/bin/bash";
        let region = PlatformMemoryPatcher::parse_maps_line(line).unwrap();
        assert_eq!(region.start, 0x400000);
        assert_eq!(region.end, 0x410000);
        assert!(region.is_readable);
        assert!(!region.is_writable);
        assert!(region.perms.execute);
    }

    #[test]
    fn test_patcher_creation() {
        let patcher = PlatformMemoryPatcher::new(1234);
        assert!(patcher.is_ok());
        if let Ok(p) = patcher {
            assert_eq!(p.pid, 1234);
        }
    }

    #[test]
    fn test_mem_path() {
        let patcher = PlatformMemoryPatcher::new(1234).unwrap();
        assert_eq!(patcher.mem_path(), "/proc/1234/mem");
        assert_eq!(patcher.maps_path(), "/proc/1234/maps");
    }
}
