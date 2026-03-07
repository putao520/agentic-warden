//! # 运行时内存补丁模块
//! 通过修改进程内存修复 BUG，不影响系统文件。
//!
//! 提供跨平台的内存补丁接口，支持 Linux、macOS 和 Windows。

pub mod error;
pub mod platform;
mod runtime;

pub use error::{PatchError, PatchResult};
pub use platform::{MemoryPatcher, MemoryRegion, MemPerm, PlatformMemoryPatcher};
pub use runtime::RuntimePatcher;
