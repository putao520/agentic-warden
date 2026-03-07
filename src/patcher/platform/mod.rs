//! 平台抽象层 - 内存补丁
//!
//! 提供跨平台的内存读写接口，支持 Linux、macOS 和 Windows。

use crate::patcher::error::PatchError;

#[cfg_attr(target_os = "linux", path = "unix.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod platform;

pub use platform::PlatformMemoryPatcher;

/// 内存区域信息
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// 起始地址
    pub start: usize,
    /// 结束地址
    pub end: usize,
    /// 权限
    pub perms: MemPerm,
    /// 是否可读
    pub is_readable: bool,
    /// 是否可写
    pub is_writable: bool,
}

/// 内存权限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemPerm {
    /// 可读
    pub read: bool,
    /// 可写
    pub write: bool,
    /// 可执行
    pub execute: bool,
}

impl MemPerm {
    /// 创建新的内存权限
    pub const fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }

    /// 是否可读写
    pub const fn is_rw(&self) -> bool {
        self.read && self.write
    }

    /// 从 Linux /proc/pid/maps 权限字符串解析
    #[cfg(unix)]
    pub(crate) fn from_maps_str(s: &str) -> Option<Self> {
        if s.len() < 3 {
            return None;
        }
        let read = s.chars().nth(0) == Some('r');
        let write = s.chars().nth(1) == Some('w');
        let execute = s.chars().nth(2) == Some('x');
        Some(Self::new(read, write, execute))
    }

    /// 从 Windows MEMORY_BASIC_INFORMATION Protection 标志解析
    #[cfg(windows)]
    pub(crate) fn from_win_prot(prot: u32) -> Self {
        use windows::Win32::System::Memory::{
            PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY,
            PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY,
        };

        match prot & 0xFF {
            PAGE_NOACCESS => Self::new(false, false, false),
            PAGE_READONLY => Self::new(true, false, false),
            PAGE_READWRITE => Self::new(true, true, false),
            PAGE_WRITECOPY => Self::new(true, true, false),
            PAGE_EXECUTE => Self::new(false, false, true),
            PAGE_EXECUTE_READ => Self::new(true, false, true),
            PAGE_EXECUTE_READWRITE => Self::new(true, true, true),
            PAGE_EXECUTE_WRITECOPY => Self::new(true, true, true),
            _ => Self::new(false, false, false),
        }
    }
}

/// 平台无关的内存补丁接口
///
/// 此 trait 定义了跨平台的内存操作方法，每个平台需要实现此 trait。
pub trait MemoryPatcher {
    /// 创建针对指定进程的补丁器
    fn new(pid: u32) -> Result<Self, PatchError>
    where
        Self: Sized;

    /// 检查进程是否存在
    fn process_exists(&self) -> bool;

    /// 读取进程的所有内存区域
    fn read_memory_maps(&self) -> Result<Vec<MemoryRegion>, PatchError>;

    /// 读取进程内存
    fn read_memory(&self, addr: usize, buf: &mut [u8]) -> Result<(), PatchError>;

    /// 写入进程内存
    fn write_memory(&self, addr: usize, data: &[u8]) -> Result<(), PatchError>;
}

/// 类型别名，指向当前平台的内存补丁器实现
pub type PlatformMemoryPatcherImpl = PlatformMemoryPatcher;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_perm_new() {
        let perm = MemPerm::new(true, true, false);
        assert!(perm.read);
        assert!(perm.write);
        assert!(!perm.execute);
        assert!(perm.is_rw());
    }

    #[test]
    fn test_mem_perm_is_rw() {
        assert!(MemPerm::new(true, true, false).is_rw());
        assert!(!MemPerm::new(true, false, false).is_rw());
        assert!(!MemPerm::new(false, true, false).is_rw());
    }

    #[test]
    fn test_memory_region() {
        let region = MemoryRegion {
            start: 0x1000,
            end: 0x2000,
            perms: MemPerm::new(true, true, false),
            is_readable: true,
            is_writable: true,
        };
        assert_eq!(region.start, 0x1000);
        assert_eq!(region.end, 0x2000);
        assert!(region.is_readable);
        assert!(region.is_writable);
    }

    #[cfg(unix)]
    #[test]
    fn test_mem_perm_from_maps_str() {
        let perm = MemPerm::from_maps_str("rw-p").unwrap();
        assert!(perm.read);
        assert!(perm.write);
        assert!(!perm.execute);

        let perm = MemPerm::from_maps_str("r-xp").unwrap();
        assert!(perm.read);
        assert!(!perm.write);
        assert!(perm.execute);

        assert!(MemPerm::from_maps_str("r").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn test_mem_perm_from_win_prot() {
        use windows::Win32::System::Memory::PAGE_READWRITE;

        let perm = MemPerm::from_win_prot(PAGE_READWRITE);
        assert!(perm.read);
        assert!(perm.write);
        assert!(!perm.execute);
    }
}
