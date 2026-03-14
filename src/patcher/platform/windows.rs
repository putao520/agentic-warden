//! Windows 平台内存补丁实现
//!
//! 使用 Windows API 实现进程内存读写:
//! - OpenProcess 打开进程
//! - VirtualQueryEx 枚举内存区域
//! - ReadProcessMemory / WriteProcessMemory 读写内存

use crate::patcher::error::{PatchError, PatchResult};
use crate::patcher::platform::{MemoryPatcher, MemoryRegion, MemPerm};
use std::ptr;
use tracing::{debug, trace, warn};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{
    MemoryBasicInformation, VirtualQueryEx, PAGE_EXECUTE_READWRITE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;

/// Windows 平台内存补丁器
pub struct PlatformMemoryPatcher {
    pid: u32,
    handle: HANDLE,
}

impl PlatformMemoryPatcher {
    /// 打开进程句柄
    fn open_process_handle(pid: u32) -> Result<HANDLE, PatchError> {
        unsafe {
            let handle = OpenProcess(
                PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_QUERY_INFORMATION,
                false,
                pid,
            )
            .map_err(|e| {
                if e.code().is_not_found() {
                    PatchError::ProcessNotFound { pid }
                } else {
                    PatchError::PermissionDenied {
                        reason: format!("Failed to open process {}: {}", pid, e),
                    }
                }
            })?;

            if handle.is_invalid() {
                return Err(PatchError::ProcessNotFound { pid });
            }

            Ok(handle)
        }
    }

    /// 检查进程是否存在
    fn check_process_exists(handle: HANDLE) -> bool {
        !handle.is_invalid()
    }

    /// 枚举内存区域
    fn enumerate_memory_regions(&self) -> PatchResult<Vec<MemoryRegion>> {
        let mut regions = Vec::new();
        let mut address = 0usize;

        unsafe {
            loop {
                let mut mem_info: MemoryBasicInformation = std::mem::zeroed();
                let result = VirtualQueryEx(
                    self.handle,
                    address as *const _,
                    &mut mem_info,
                    std::mem::size_of::<MemoryBasicInformation>(),
                );

                if result == 0 {
                    // 检查是否到达内存末尾
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
                        break;
                    }
                    // 其他错误，继续尝试
                    address = address.wrapping_add(0x10000);
                    if address < 0x10000 {
                        // 地址回绕，结束
                        break;
                    }
                    continue;
                }

                let region_start = mem_info.BaseAddress as usize;
                let region_size = mem_info.RegionSize;
                let region_end = region_start.wrapping_add(region_size);

                // 跳过保留区域
                if mem_info.State == windows::Win32::System::Memory::MEM_COMMIT {
                    let perms = MemPerm::from_win_prot(mem_info.Protect.0);
                    let is_readable = perms.read;
                    let is_writable = perms.write;

                    regions.push(MemoryRegion {
                        start: region_start,
                        end: region_end,
                        perms,
                        is_readable,
                        is_writable,
                    });
                }

                // 移动到下一个区域
                address = region_end;
                if address < region_start {
                    // 地址回绕，结束
                    break;
                }
            }
        }

        trace!(
            "Enumerated {} memory regions for process {}",
            regions.len(),
            self.pid
        );

        Ok(regions)
    }
}

impl Drop for PlatformMemoryPatcher {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

impl MemoryPatcher for PlatformMemoryPatcher {
    fn new(pid: u32) -> Result<Self, PatchError> {
        let handle = Self::open_process_handle(pid)?;
        Ok(Self { pid, handle })
    }

    fn process_exists(&self) -> bool {
        Self::check_process_exists(self.handle)
    }

    fn read_memory_maps(&self) -> PatchResult<Vec<MemoryRegion>> {
        self.enumerate_memory_regions()
    }

    fn read_memory(&self, addr: usize, buf: &mut [u8]) -> PatchResult<()> {
        if buf.is_empty() {
            return Ok(());
        }

        unsafe {
            let mut bytes_read = 0;
            let success = ReadProcessMemory(
                self.handle,
                addr as *const _,
                buf.as_mut_ptr() as *mut _,
                buf.len(),
                Some(&mut bytes_read),
            )
            .is_ok();

            if !success {
                return Err(PatchError::ReadFailed {
                    reason: format!(
                        "Failed to read {} bytes from address {:x}",
                        buf.len(),
                        addr
                    ),
                });
            }

            if bytes_read != buf.len() {
                warn!(
                    "Partial read: {} bytes requested, {} bytes read from address {:x}",
                    buf.len(),
                    bytes_read,
                    addr
                );
            }

            trace!("Read {} bytes from address {:x}", bytes_read, addr);
        }

        Ok(())
    }

    fn write_memory(&self, addr: usize, data: &[u8]) -> PatchResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        unsafe {
            let mut bytes_written = 0;
            let success = WriteProcessMemory(
                self.handle,
                addr as *mut _,
                data.as_ptr() as *const _,
                data.len(),
                Some(&mut bytes_written),
            )
            .is_ok();

            if !success {
                return Err(PatchError::WriteFailed {
                    reason: format!("Failed to write {} bytes to address {:x}", data.len(), addr),
                });
            }

            if bytes_written != data.len() {
                warn!(
                    "Partial write: {} bytes requested, {} bytes written to address {:x}",
                    data.len(),
                    bytes_written,
                    addr
                );
            }

            debug!("Wrote {} bytes to address {:x}", bytes_written, addr);
        }

        Ok(())
    }
}

// Windows 错误码常量
const ERROR_INVALID_PARAMETER: u32 = 87;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_perm_from_win_prot() {
        use windows::Win32::System::Memory::{
            PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_NOACCESS,
            PAGE_READONLY, PAGE_READWRITE,
        };

        assert!(!MemPerm::from_win_prot(PAGE_NOACCESS).read);
        assert!(MemPerm::from_win_prot(PAGE_READONLY).read);
        assert!(!MemPerm::from_win_prot(PAGE_READONLY).write);

        let rw = MemPerm::from_win_prot(PAGE_READWRITE);
        assert!(rw.read);
        assert!(rw.write);
        assert!(!rw.execute);

        let rx = MemPerm::from_win_prot(PAGE_EXECUTE_READ);
        assert!(rx.read);
        assert!(!rx.write);
        assert!(rx.execute);

        let rwx = MemPerm::from_win_prot(PAGE_EXECUTE_READWRITE);
        assert!(rwx.read);
        assert!(rwx.write);
        assert!(rwx.execute);
    }

    #[test]
    fn test_patcher_creation_invalid_pid() {
        // 使用不太可能存在的 PID
        let result = PlatformMemoryPatcher::new(999999);
        // 应该失败，因为进程不存在
        assert!(result.is_err());
    }
}
