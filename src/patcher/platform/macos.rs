//! macOS 平台内存补丁实现（使用 mach API）
//!
//! 使用 task_for_pid 获取任务端口，通过 vm_read_overwrite 和 vm_write
//! 进行内存读写操作。

use crate::patcher::error::{PatchError, PatchResult};
use crate::patcher::platform::{MemoryPatcher, MemoryRegion, MemPerm};
use std::ffi::c_void;
use tracing::{debug, trace};

// mach 类型定义
type mach_port_t = u32;
type vm_address_t = usize;
type vm_size_t = usize;
type vm_offset_t = usize;

// mach 返回码
type kern_return_t = i32;

// 常量
const VM_PROT_READ: i32 = 1;
const VM_PROT_WRITE: i32 = 2;
const VM_PROT_EXECUTE: i32 = 4;

/// Mach API 错误码转人类可读消息
fn mach_error_string(code: kern_return_t) -> &'static str {
    match code {
        1 => "KERN_INVALID_ADDRESS - Invalid address",
        2 => "KERN_PROTECTION_FAILURE - Protection failure",
        3 => "KERN_NO_SPACE - No space available",
        4 => "KERN_INVALID_ARGUMENT - Invalid argument",
        5 => "KERN_FAILURE - Failure",
        6 => "KERN_RESOURCE_SHORTAGE - Resource shortage",
        8 => "KERN_MEMORY_ERROR - Memory error",
        9 => "KERN_MEMORY_FAILURE - Memory failure",
        10 => "KERN_NOT_RECEIVER - Not receiver",
        11 => "KERN_NO_ACCESS - No access",
        12 => "KERN_FAILURE - Failure",
        13 => "KERN_MEMORY_FAILURE - Memory failure",
        14 => "KERN_ALREADY_IN_SET - Already in set",
        15 => "KERN_NOT_IN_SET - Not in set",
        16 => "KERN_NAME_EXISTS - Name exists",
        17 => "KERN_ABORTED - Aborted",
        18 => "KERN_INVALID_NAME - Invalid name",
        19 => "KERN_INVALID_TASK - Invalid task",
        20 => "KERN_INVALID_RIGHT - Invalid right",
        21 => "KERN_INVALID_VALUE - Invalid value",
        22 => "KERN_UREFS_OVERFLOW - Urefs overflow",
        23 => "KERN_INVALID_CAPABILITY - Invalid capability",
        24 => "KERN_RIGHT_EXISTS - Right exists",
        25 => "KERN_INVALID_HOST - Invalid host",
        26 => "KERN_MEMORY_PRESENT - Memory present",
        27 => "KERN_MEMORY_DATA_MOVED - Memory data moved",
        28 => "KERN_NOT_SUPPORTED - Not supported",
        _ => "Unknown mach error",
    }
}

// FFI 绑定到 mach 函数
extern "C" {
    /// 获取当前任务的端口
    fn mach_task_self() -> mach_port_t;

    /// 获取指定 PID 的任务端口
    fn task_for_pid(
        target_task: mach_port_t,
        pid: i32,
        task: *mut mach_port_t,
    ) -> kern_return_t;

    /// 读取进程内存
    fn vm_read_overwrite(
        target_task: mach_port_t,
        address: vm_address_t,
        size: vm_size_t,
        data: vm_address_t,
        out_size: *mut vm_size_t,
    ) -> kern_return_t;

    /// 写入进程内存
    fn vm_write(
        target_task: mach_port_t,
        address: vm_address_t,
        data: vm_address_t,
        size: vm_size_t,
    ) -> kern_return_t;

    /// 修改内存保护
    fn vm_protect(
        target_task: mach_port_t,
        address: vm_address_t,
        size: vm_size_t,
        set_maximum: i32,
        new_prot: i32,
    ) -> kern_return_t;

    /// 释放 mach 端口
    fn mach_port_deallocate(
        task: mach_port_t,
        name: mach_port_t,
    ) -> kern_return_t;
}

/// macOS 内存补丁器
pub struct PlatformMemoryPatcher {
    pid: u32,
    task: mach_port_t,
}

impl PlatformMemoryPatcher {
    /// 获取任务端口
    fn get_task_port(pid: u32) -> PatchResult<mach_port_t> {
        unsafe {
            let mut task: mach_port_t = 0;
            let result = task_for_pid(mach_task_self(), pid as i32, &mut task);

            if result != 0 {
                return Err(PatchError::PermissionDenied {
                    reason: format!(
                        "task_for_pid failed (code {}): {}. \
                        This requires either:\n\
                        - codesigning with --entitlements (get-task-allow)\n\
                        - sudo/root privileges\n\
                        - SIP disabled (not recommended)",
                        result,
                        mach_error_string(result)
                    ),
                });
            }

            if task == 0 {
                return Err(PatchError::ProcessNotFound { pid });
            }

            Ok(task)
        }
    }

    /// 修改内存保护（使内存可写）
    #[allow(dead_code)]
    fn make_writable(&self, addr: usize, size: usize) -> PatchResult<()> {
        unsafe {
            // 首先尝试读取当前保护状态
            let result = vm_protect(
                self.task,
                addr as vm_address_t,
                size as vm_size_t,
                0, // set_maximum = FALSE
                VM_PROT_READ | VM_PROT_WRITE | VM_PROT_EXECUTE,
            );

            if result != 0 && result != 2 {
                // KERN_PROTECTION_FAILURE (2) 可以忽略，尝试继续
                trace!(
                    "vm_protect at {:x} returned: {}",
                    addr,
                    mach_error_string(result)
                );
            }

            Ok(())
        }
    }

    /// 检查进程是否存在
    fn check_process_exists(&self) -> bool {
        // 使用 ps 命令检查
        std::process::Command::new("ps")
            .arg("-p")
            .arg(self.pid.to_string())
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// 解析 vmmap 输出
    fn parse_vmmap_output(output: &str) -> PatchResult<Vec<MemoryRegion>> {
        let mut regions = Vec::new();

        for line in output.lines() {
            // 跳过非数据行
            if line.is_empty()
                || line.starts_with("====")
                || line.starts_with("Usage")
                || line.contains("PID=")
            {
                continue;
            }

            if let Some(region) = Self::parse_vmmap_line(line) {
                regions.push(region);
            }
        }

        Ok(regions)
    }

    /// 解析单行 vmmap 输出
    fn parse_vmmap_line(line: &str) -> Option<MemoryRegion> {
        // vmmap 输出格式示例:
        // __TEXT                 0000000100000000-0000000100001000 [    4K] r-x/rwx SM=COW  ...
        // 或更简单的格式:
        // 100000000-100001000    r-x/rwx ...

        // 查找地址范围
        let addr_start = line.find("0x")?;
        let addr_part: String = line
            .chars()
            .skip(addr_start)
            .take_while(|c| c.is_ascii_hexdigit() || c == '-' || c == 'x')
            .collect();

        if let Some(dash_pos) = addr_part.find('-') {
            let start_str = &addr_part[2..dash_pos]; // 跳过 "0x"
            let end_str = &addr_part[dash_pos + 1..];

            if let (Ok(start), Ok(end)) = (
                usize::from_str_radix(start_str, 16),
                usize::from_str_radix(end_str, 16),
            ) {
                // 查找权限: r-x/rwx 或 rwx 等格式
                let line_lower = line.to_lowercase();

                // 检查权限字符
                let read = line_lower.contains('r');
                let write = line_lower.contains('w');
                let execute = line_lower.contains('x');

                return Some(MemoryRegion {
                    start,
                    end,
                    perms: MemPerm::new(read, write, execute),
                    is_readable: read,
                    is_writable: write,
                });
            }
        }

        None
    }
}

impl MemoryPatcher for PlatformMemoryPatcher {
    fn new(pid: u32) -> Result<Self, PatchError> {
        let task = Self::get_task_port(pid)?;
        debug!("Successfully acquired task port for PID {}", pid);
        Ok(Self { pid, task })
    }

    fn process_exists(&self) -> bool {
        self.check_process_exists()
    }

    fn read_memory_maps(&self) -> PatchResult<Vec<MemoryRegion>> {
        // 使用 vmmap 命令获取内存映射
        let output = std::process::Command::new("vmmap")
            .arg("-w")  // 宽输出格式
            .arg(self.pid.to_string())
            .output()
            .map_err(|e| PatchError::ReadFailed {
                reason: format!("Failed to execute vmmap: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(if stderr.contains("cannot examine") || stderr.contains("no such process") {
                PatchError::ProcessNotFound { pid: self.pid }
            } else {
                PatchError::ReadFailed {
                    reason: format!("vmmap failed: {}", stderr),
                }
            });
        }

        Self::parse_vmmap_output(&String::from_utf8_lossy(&output.stdout))
    }

    fn read_memory(&self, addr: usize, buf: &mut [u8]) -> PatchResult<()> {
        unsafe {
            let mut size = buf.len() as vm_size_t;
            let result = vm_read_overwrite(
                self.task,
                addr as vm_address_t,
                buf.len() as vm_size_t,
                buf.as_ptr() as vm_address_t,
                &mut size,
            );

            if result != 0 {
                return Err(PatchError::ReadFailed {
                    reason: format!(
                        "vm_read_overwrite failed at {:x} (code {}): {}",
                        addr,
                        result,
                        mach_error_string(result)
                    ),
                });
            }

            if size != buf.len() as vm_size_t {
                return Err(PatchError::ReadFailed {
                    reason: format!(
                        "Partial read: got {} bytes, expected {} bytes",
                        size,
                        buf.len()
                    ),
                });
            }

            trace!("Read {} bytes from address {:x}", buf.len(), addr);
            Ok(())
        }
    }

    fn write_memory(&self, addr: usize, data: &[u8]) -> PatchResult<()> {
        unsafe {
            let result = vm_write(
                self.task,
                addr as vm_address_t,
                data.as_ptr() as vm_address_t,
                data.len() as vm_size_t,
            );

            if result != 0 {
                // 尝试修改内存保护后再写入
                if result == 2 {
                    // KERN_PROTECTION_FAILURE
                    self.make_writable(addr, data.len())?;

                    // 重试写入
                    let retry_result = vm_write(
                        self.task,
                        addr as vm_address_t,
                        data.as_ptr() as vm_address_t,
                        data.len() as vm_size_t,
                    );

                    if retry_result != 0 {
                        return Err(PatchError::WriteFailed {
                            reason: format!(
                                "vm_write failed at {:x} after vm_protect (code {}): {}",
                                addr,
                                retry_result,
                                mach_error_string(retry_result)
                            ),
                        });
                    }
                } else {
                    return Err(PatchError::WriteFailed {
                        reason: format!(
                            "vm_write failed at {:x} (code {}): {}",
                            addr,
                            result,
                            mach_error_string(result)
                        ),
                    });
                }
            }

            debug!("Wrote {} bytes to address {:x}", data.len(), addr);
            Ok(())
        }
    }
}

impl Drop for PlatformMemoryPatcher {
    fn drop(&mut self) {
        // 释放任务端口
        if self.task != 0 {
            unsafe {
                let _ = mach_port_deallocate(mach_task_self(), self.task);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmmap_parsing() {
        let line = "__TEXT                 0000000100000000-0000000100001000 [    4K] r-x/rwx SM=COW  /usr/bin/test";
        let region = PlatformMemoryPatcher::parse_vmmap_line(line);
        assert!(region.is_some());
        let r = region.unwrap();
        assert_eq!(r.start, 0x100000000);
        assert_eq!(r.end, 0x100001000);
        assert!(r.is_readable);
        assert!(!r.is_writable);
        assert!(r.perms.execute);
    }

    #[test]
    fn test_vmmap_parsing_writable() {
        let line = "__DATA                 0000000100001000-0000000100002000 [    4K] rw-/rwx SM=PRV  /usr/bin/test";
        let region = PlatformMemoryPatcher::parse_vmmap_line(line);
        assert!(region.is_some());
        let r = region.unwrap();
        assert_eq!(r.start, 0x10001000);
        assert_eq!(r.end, 0x10002000);
        assert!(r.is_readable);
        assert!(r.is_writable);
        assert!(!r.perms.execute);
    }

    #[test]
    fn test_vmmap_parsing_simple() {
        let line = "100000000-100001000    r-x    /usr/bin/test";
        let region = PlatformMemoryPatcher::parse_vmmap_line(line);
        assert!(region.is_some());
        let r = region.unwrap();
        assert_eq!(r.start, 0x100000000);
        assert_eq!(r.end, 0x100001000);
        assert!(r.is_readable);
        assert!(!r.is_writable);
        assert!(r.perms.execute);
    }

    #[test]
    fn test_vmmap_parsing_invalid() {
        // 无效行，没有地址
        let line = "==== ==== ==== ====";
        let region = PlatformMemoryPatcher::parse_vmmap_line(line);
        assert!(region.is_none());
    }

    #[test]
    fn test_vmmap_parsing_no_perms() {
        // 有地址但没有明确权限标记
        let line = "0000000100000000-0000000100001000 [    4K] SM=COW  /usr/bin/test";
        let region = PlatformMemoryPatcher::parse_vmmap_line(line);
        // 应该返回一个区域，只是权限可能不正确
        assert!(region.is_some());
    }

    #[test]
    fn test_vmmap_parse_full_output() {
        let output = r#"==== ====
==== ====
__TEXT                 0000000100000000-0000000100001000 [    4K] r-x/rwx SM=COW  /usr/bin/test
__DATA                 0000000100001000-0000000100002000 [    4K] rw-/rwx SM=PRV  /usr/bin/test
==== ==== "#;

        let regions = PlatformMemoryPatcher::parse_vmmap_output(output).unwrap();
        assert_eq!(regions.len(), 2);

        assert_eq!(regions[0].start, 0x100000000);
        assert_eq!(regions[0].end, 0x100001000);
        assert!(regions[0].is_readable);
        assert!(!regions[0].is_writable);

        assert_eq!(regions[1].start, 0x10001000);
        assert_eq!(regions[1].end, 0x10002000);
        assert!(regions[1].is_readable);
        assert!(regions[1].is_writable);
    }
}
