//! # 统一补丁框架
//!
//! 支持文件补丁和内存补丁两种模式，通过通用 regex 跨版本匹配 Claude CLI
//! 常量块，无需维护版本签名数据库。
//!
//! ## 补丁类型
//!
//! - **文件补丁**: 直接修改磁盘文件，持久化，一次性
//! - **内存补丁**: 运行时修改进程内存，每次启动应用
//!
//! ## 功能类型
//!
//! - `MaxContextTokens`: 可配置默认上下文窗口 + autoCompact 阈值

pub mod error;
pub mod file;
pub mod platform;
pub mod registry;
pub mod runtime;
pub mod types;
pub mod versions;

pub use error::{PatchError, PatchResult};
pub use platform::{MemoryPatcher, MemoryRegion, MemPerm, PlatformMemoryPatcher};
pub use runtime::RuntimePatcher;
pub use types::{
    FeatureType, PatchType, UnifiedPatchPattern, UnifiedPatchResult,
    UnifiedPatchError, Result as PatchResultType,
};
pub use registry::get_feature_patches;
pub use file::{apply_file_patch, is_file_patched, get_claude_cli_path, get_claude_js_path, InstallationType, detect_installation, get_patchable_path, restore_from_backup};
