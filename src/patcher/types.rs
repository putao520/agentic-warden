//! 统一补丁框架核心类型定义

use serde::{Deserialize, Serialize};

/// 补丁类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatchType {
    /// 文件补丁 - 直接修改磁盘文件
    File,
    /// 内存补丁 - 运行时修改进程内存
    Memory,
}

/// 功能类型 - 每个需要补丁的功能
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeatureType {
    /// ToolSearch 功能解锁
    ToolSearch,
    /// UltraThink/Effort 功能解锁
    UltraThink,
    /// AgentTeams 功能解锁
    AgentTeams,
    /// WebSearch 地区限制绕过
    WebSearch,
    /// 持久代理内存
    PersistentMemory,
}

impl FeatureType {
    /// 获取功能的描述
    pub fn description(&self) -> &'static str {
        match self {
            FeatureType::ToolSearch => "ToolSearch - 工具搜索功能解锁",
            FeatureType::UltraThink => "UltraThink - 思考模式完整功能解锁",
            FeatureType::AgentTeams => "AgentTeams - Agent 团队功能",
            FeatureType::WebSearch => "WebSearch - 网络搜索地区限制绕过",
            FeatureType::PersistentMemory => "PersistentMemory - 持久代理内存",
        }
    }

    /// 获取功能的简短名称
    pub fn short_name(&self) -> &'static str {
        match self {
            FeatureType::ToolSearch => "toolsearch",
            FeatureType::UltraThink => "ultrathink",
            FeatureType::AgentTeams => "agentteams",
            FeatureType::WebSearch => "websearch",
            FeatureType::PersistentMemory => "persistent",
        }
    }
}

impl std::fmt::Display for FeatureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// 功能补丁模式 - 支持文件和内存两种补丁
#[derive(Debug, Clone)]
pub struct UnifiedPatchPattern {
    /// 功能类型
    pub feature: FeatureType,
    /// 补丁类型
    pub patch_type: PatchType,
    /// 搜索模式（字节序列或字符串）
    pub search_pattern: &'static [u8],
    /// 替换模式（用于文件补丁）
    pub replace_pattern: Option<&'static [u8]>,
    /// 内存补丁：单个字节替换
    pub patch_byte: Option<u8>,
    /// 内存补丁：替换位置偏移
    pub patch_offset: Option<usize>,
    /// 描述
    pub description: &'static str,
}

/// 补丁应用结果
#[derive(Debug, Clone)]
pub enum UnifiedPatchResult {
    /// 文件补丁成功
    FilePatched { path: String },
    /// 内存补丁成功
    MemoryPatched { address: usize, pid: u32 },
    /// 跳过（不需要补丁）
    Skipped { reason: String },
}

impl std::fmt::Display for UnifiedPatchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnifiedPatchResult::FilePatched { path } => {
                write!(f, "文件补丁成功: {}", path)
            }
            UnifiedPatchResult::MemoryPatched { address, pid } => {
                write!(f, "内存补丁成功: PID={}, 地址={:#x}", pid, address)
            }
            UnifiedPatchResult::Skipped { reason } => {
                write!(f, "跳过: {}", reason)
            }
        }
    }
}

/// 统一补丁错误类型
#[derive(Debug, thiserror::Error)]
pub enum UnifiedPatchError {
    #[error("文件操作失败: {0}")]
    FileError(#[from] std::io::Error),

    #[error("内存操作失败: {0}")]
    MemoryError(String),

    #[error("不支持的功能: {0:?}")]
    UnsupportedFeature(FeatureType),

    #[error("不支持的补丁类型: {0:?}")]
    UnsupportedPatchType(PatchType),

    #[error("版本不支持: {0}")]
    VersionNotSupported(String),

    #[error("模式未找到: {0}")]
    PatternNotFound(String),

    #[error("进程未找到: PID {pid}")]
    ProcessNotFound { pid: u32 },

    #[error("权限被拒绝: {0}")]
    PermissionDenied(String),

    #[error("其他错误: {0}")]
    Other(String),
}

impl From<crate::patcher::error::PatchError> for UnifiedPatchError {
    fn from(err: crate::patcher::error::PatchError) -> Self {
        match err {
            crate::patcher::error::PatchError::ProcessNotFound { pid } => {
                UnifiedPatchError::ProcessNotFound { pid }
            }
            crate::patcher::error::PatchError::PatternNotFound { pattern, .. } => {
                UnifiedPatchError::PatternNotFound(pattern)
            }
            crate::patcher::error::PatchError::PermissionDenied { reason } => {
                UnifiedPatchError::PermissionDenied(reason)
            }
            crate::patcher::error::PatchError::ReadFailed { reason }
            | crate::patcher::error::PatchError::WriteFailed { reason } => {
                UnifiedPatchError::MemoryError(reason)
            }
            crate::patcher::error::PatchError::Io(io) => UnifiedPatchError::FileError(io),
            _ => UnifiedPatchError::Other(err.to_string()),
        }
    }
}

/// 统一补丁结果类型
pub type Result<T> = std::result::Result<T, UnifiedPatchError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_type_descriptions() {
        assert_eq!(FeatureType::ToolSearch.short_name(), "toolsearch");
        assert_eq!(FeatureType::UltraThink.short_name(), "ultrathink");
        assert_eq!(FeatureType::AgentTeams.short_name(), "agentteams");
        assert_eq!(FeatureType::WebSearch.short_name(), "websearch");
        assert_eq!(FeatureType::PersistentMemory.short_name(), "persistent");
    }
}
