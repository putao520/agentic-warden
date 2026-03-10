//! # 统一补丁框架
//!
//! 支持文件补丁和内存补丁两种模式，按版本管理不同 Claude CLI 版本的补丁。
//!
//! ## 补丁类型
//!
//! - **文件补丁**: 直接修改磁盘文件，持久化，一次性
//! - **内存补丁**: 运行时修改进程内存，每次启动应用
//!
//! ## 功能类型
//!
//! - `ToolSearch`: 工具搜索功能解锁
//! - `UltraThink`: 思考模式完整功能解锁
//! - `AgentTeams`: Agent 团队功能
//! - `WebSearch`: 网络搜索地区限制绕过
//! - `PersistentMemory`: 持久代理内存

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
pub use file::{apply_file_patch, is_file_patched, get_claude_cli_path, get_claude_js_path};
