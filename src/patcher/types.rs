//! 统一补丁框架核心类型定义

use serde::{Deserialize, Serialize};
use std::borrow::Cow;

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
    /// MaxContextTokens - 可配置默认上下文窗口 + autoCompact 阈值
    ///
    /// 通过 regex 通用模式匹配 Claude CLI 的常量块
    /// `var X=200000,Y=200000,...`，把两个 200000 替换为配置值。
    MaxContextTokens,
}

impl FeatureType {
    /// 获取功能的描述
    pub fn description(&self) -> &'static str {
        match self {
            FeatureType::MaxContextTokens => "MaxContextTokens - 可配置默认上下文窗口 + autoCompact 阈值",
        }
    }

    /// 获取功能的简短名称
    pub fn short_name(&self) -> &'static str {
        match self {
            FeatureType::MaxContextTokens => "maxtokens",
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
    ///
    /// 当 `use_regex=true` 时，此字段存放 regex 字符串的字节表示，
    /// 运行时通过 `regex::bytes::Regex` 编译并扫描。
    pub search_pattern: Cow<'static, [u8]>,
    /// 替换模式（用于文件补丁）
    ///
    /// 当 `use_regex=true` 时为 None，替换值由 `regex_replace_values`
    /// 在运行时动态构造（按顺序替换匹配文本里的数字字面量）。
    pub replace_pattern: Option<Cow<'static, [u8]>>,
    /// 内存补丁：单个字节替换
    pub patch_byte: Option<u8>,
    /// 内存补丁：替换位置偏移
    pub patch_offset: Option<usize>,
    /// 描述
    pub description: Cow<'static, str>,
    /// 是否将 search_pattern 作为 regex 处理
    ///
    /// true 时 search_pattern 作为 regex 字符串，replace_pattern 为 None，
    /// 通过 `regex_replace_values` 顺序替换匹配文本中的数字。
    pub use_regex: bool,
    /// regex 模式下的顺序替换值
    ///
    /// 例如匹配到 `var X=200000,Y=200000,...` 后，
    /// `regex_replace_values=Some(vec![500000, 500000])` 会把
    /// 第一个 200000 替换为 500000，第二个 200000 替换为 500000。
    /// 仅在 `use_regex=true` 时生效。
    pub regex_replace_values: Option<Vec<u32>>,
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
    fn test_max_context_tokens_description() {
        assert!(FeatureType::MaxContextTokens
            .description()
            .contains("MaxContextTokens"));
    }

    #[test]
    fn test_max_context_tokens_short_name() {
        assert_eq!(FeatureType::MaxContextTokens.short_name(), "maxtokens");
    }

    #[test]
    fn test_cow_pattern_construction() {
        // 验证 Cow 字段可用字面量构造
        let pattern = UnifiedPatchPattern {
            feature: FeatureType::MaxContextTokens,
            patch_type: PatchType::Memory,
            search_pattern: b"var YOt=200000".as_ref().into(),
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: "test".into(),
            use_regex: false,
            regex_replace_values: None,
        };
        assert_eq!(pattern.search_pattern.as_ref(), b"var YOt=200000");
        assert!(!pattern.use_regex);
    }
}
