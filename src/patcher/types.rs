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
    /// AntiTelemetry - 截断 CC 客户端上报（event_logging 端点 → 404）
    ///
    /// 通过字面量替换把 `/api/event_logging/v2/batch` 改成
    /// `/api/event_logging/v2/xxxxx`，让上报端点 404 静默失败。
    /// 跨版本稳定（API 路径字面量）。
    AntiTelemetry,
    /// AntiSpy - 时区+中转站识别失明（KIt→UTC, Hsp→null）
    ///
    /// 通过函数级等长字面量替换让 CC 本地识别全失明：
    /// - `KIt()` → 返回 `"UTC"`：时区永远返回 UTC，真实时区不泄露
    /// - `Hsp()` → 返回 `null`：中转站识别返回 null，known/labKw/cnTZ/host 全 null
    ///
    /// 不碰 `$Sn()`（保留 firstParty 专属功能）。跨版本稳定（函数体字面量）。
    AntiSpy,
    /// AntiPromptBias - 消除 Provider context 提示词偏见（第三方不再被注入"功能有差异"提示）
    ///
    /// 通过等长字面量替换把第三方用户的 Provider context prompt 注入条件
    /// `if(g7())` 改成 `if(0   )`，让条件永远 false → 该条 prompt 不注入，
    /// 模型不感知 provider 差异，行为更一致。
    /// 只跳过这一条 prompt，不影响其他 firstParty 门控（OAuth/能力/模型选择等照常）。
    /// 跨版本稳定（prompt 字面量，非 minified 变量名）。
    AntiPromptBias,
    /// AntiAtis - 防止 x-cc-atis 追踪 header 注入（atis 提取函数 → void 0）
    ///
    /// 逃生口短路 patch 副作用：gu()=true 激活 tMi(firstParty)&&gu() 条件，
    /// 触发 x-cc-atis header 注入（服务端 bootstrap 下发的追踪 token）。
    /// patch atis 提取函数让它永远返回 void 0，header 永不注入。
    /// 跨 196-199 通用（195 无此机制），语义正则通配函数名。
    AntiAtis,
    /// AntiFrameTrack - 截断 /api/frame/track 第二上报通道（绕过 AntiTelemetry 的独立 frame 服务上报）
    ///
    /// `trackFrameEvent(trr)` 函数上报 artifact 使用行为（`frame_surfaced`
    /// 事件 + slug/via/mode + X-Frame-* header）到 `BASE_API_URL` 的
    /// `/api/frame/track` 端点。这是独立于 AntiTelemetry 的第二上报通道
    /// （AntiTelemetry 只截断 `/api/event_logging/v2/batch`）。
    /// 通过等长字面量替换把端点改成 `/api/frame/xxxxx` → 404 静默失败
    /// （try/catch 吞错）。跨版本稳定（API 路径字面量）。
    AntiFrameTrack,
    /// AntiCloudDetect - 禁用 MAC 地址 GCE 云检测（/^42:01/ → /^00:00/，tMi 永远 false）
    ///
    /// `tMi()` 函数遍历 `networkInterfaces()` 的 MAC 地址，用 `fGd=/^42:01/`
    /// regex 匹配 GCE 实例 OUI 前缀（Google 的 MAC 厂商前缀）。当前是
    /// 预留间谍点（导出但无内部调用方），防未来版本激活。通过等长字面量
    /// 替换把 regex 改成 `/^00:00/`（永不匹配任何 MAC）→ `fGd.test()` 永远
    /// false → `tMi()` 永远返回 false。跨版本稳定（regex 字面量）。
    /// 不防 eMi（GCE BIOS `/Google/.test`）：eMi Linux-only + 需读文件,
    /// 中转站用户通常不跑 GCE，且 `/Google/` 字面量太通用不能改。
    AntiCloudDetect,
    /// GrokAntiRepoBundle - 禁用 Grok Repo Changes git bundle 上传（call→xor+mov）
    ///
    /// patch GCS blob 上传 dispatcher 的 2 个调用点（call 指令），
    /// 等长替换 e8 xx xx xx xx → 31 c0 48 89 07（xor eax,eax; mov [rdi],rax），
    /// 让上传函数返回空结果，调用方走"无结果→跳过"分支。跨版本稳定
    /// （tracing 字符串 + call 字节模式动态定位）。
    GrokAntiRepoBundle,
    /// GrokAntiDeployUpload - 禁用 Grok App Builder 部署上传（call→xor+mov）
    GrokAntiDeployUpload,
    /// GrokAntiTraceUpload - 禁用 Grok Session Trace 上传（call→xor+mov）
    GrokAntiTraceUpload,
}

impl FeatureType {
    /// 获取功能的描述
    pub fn description(&self) -> &'static str {
        match self {
            FeatureType::MaxContextTokens => "MaxContextTokens - 可配置默认上下文窗口 + autoCompact 阈值",
            FeatureType::AntiTelemetry => "AntiTelemetry - 截断客户端上报（event_logging → 404）",
            FeatureType::AntiSpy => "AntiSpy - 时区+中转站识别失明（本地识别全 null）",
            FeatureType::AntiPromptBias => {
                "AntiPromptBias - 消除 Provider context 提示词偏见（if(g7())→if(0)）"
            }
            FeatureType::AntiAtis => {
                "AntiAtis - 防止 x-cc-atis 追踪 header 注入（atis 提取 → void 0）"
            }
            FeatureType::AntiFrameTrack => {
                "AntiFrameTrack - 截断 frame/track 第二上报通道（端点 → 404）"
            }
            FeatureType::AntiCloudDetect => {
                "AntiCloudDetect - 禁用 MAC 地址 GCE 云检测（/^42:01/ → /^00:00/）"
            }
            FeatureType::GrokAntiRepoBundle => "GrokAntiRepoBundle - 禁用 Repo Changes git bundle 上传（call→xor+mov）",
            FeatureType::GrokAntiDeployUpload => "GrokAntiDeployUpload - 禁用 App Builder 部署上传",
            FeatureType::GrokAntiTraceUpload => "GrokAntiTraceUpload - 禁用 Session Trace 上传",
        }
    }

    /// 获取功能的简短名称
    pub fn short_name(&self) -> &'static str {
        match self {
            FeatureType::MaxContextTokens => "maxtokens",
            FeatureType::AntiTelemetry => "antitelemetry",
            FeatureType::AntiSpy => "antispy",
            FeatureType::AntiPromptBias => "antipromptbias",
            FeatureType::AntiAtis => "antiatis",
            FeatureType::AntiFrameTrack => "antiframetrack",
            FeatureType::AntiCloudDetect => "anticloudetect",
            FeatureType::GrokAntiRepoBundle => "grokantirepobundle",
            FeatureType::GrokAntiDeployUpload => "grokantideployupload",
            FeatureType::GrokAntiTraceUpload => "grokantitraceupload",
        }
    }
}

impl std::fmt::Display for FeatureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// regex 动态替换模式（模式 4）的替换策略
///
/// 当 `UnifiedPatchPattern.use_regex=true` 且 `replace_pattern=None` 且
/// `regex_replace_values=None` 且 `dynamic_replace=Some` 时生效。
///
/// regex 匹配后，保留指定捕获组内容（语义要求保留的部分：函数名/数组变量名/
/// prompt 文本），把其余部分替换为字面量 + 空格填充至匹配长度，等长自动保证。
/// 用于跨版本 patch 点：minified 变量名长度变化导致匹配长度不固定，固定字面量
/// replace 无法满足等长约束（如 AntiPromptBias 的 FN 从 2 字符变 3 字符）。
#[derive(Debug, Clone)]
pub enum DynamicReplace {
    /// 替换前缀 + 保留后缀捕获组（AntiPromptBias 用）
    ///
    /// 结果：`prefix_literal` + 空格填充 + `match[keep_group]`
    ///
    /// 例：`if(yfe())r.push("**Provider context...")`
    ///   → `if(0)` + 空格 + `r.push("**Provider context...")`
    ///   （条件 if(FN()) → if(0) 恒假，push 语句原样保留）
    ReplacePrefix {
        /// 保留的后缀捕获组索引（regex 中用 `(...)` 捕获）
        keep_group: usize,
        /// 替换前缀的字面量（如 `b"if(0)"`）
        prefix_literal: Cow<'static, [u8]>,
    },
    /// 保留前缀捕获组 + 替换后缀（AntiAtis 用）
    ///
    /// 结果：`match[keep_group]` + `suffix_literal` + 空格填充 + `end_literal`
    ///
    /// 例：`function R0i(){let e=mL()?.atis;...void 0}`
    ///   → `function R0i(){` + `return void 0` + 空格 + `}`
    ///   （函数名保留，函数体 → return void 0）
    KeepPrefix {
        /// 保留的前缀捕获组索引（含函数名等必须保留的部分）
        keep_group: usize,
        /// 替换后缀起始字面量（如 `b"return void 0"`）
        suffix_literal: Cow<'static, [u8]>,
        /// 结尾字面量（如 `b"}"`）
        end_literal: Cow<'static, [u8]>,
    },
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
    /// 四种使用方式：
    /// - **字面量模式**（`use_regex=false`）：`search_pattern` 是字面量字节，
    ///   `replace_pattern` 是等长的字面量替换字节，整段覆盖。
    /// - **regex 字面量替换模式**（`use_regex=true` 且 `replace_pattern=Some`）：
    ///   `search_pattern` 是 regex 字符串，匹配后用 `replace_pattern` 整段
    ///   覆盖匹配文本（要求 regex 匹配长度 == `replace_pattern.len()`，等长）。
    ///   用于跨版本 patch 点（minified 变量名变化但匹配文本长度固定，
    ///   如 `if(Oe.xxx)return!0` / `if(Pe.xxx)return!0` 都 55 字节）。
    /// - **regex 数字替换模式**（`use_regex=true` 且 `replace_pattern=None`）：
    ///   匹配后由 `regex_replace_values` 在运行时动态构造替换值（按顺序
    ///   替换匹配文本里的数字字面量）。用于 max-token 的 200000→目标值替换。
    /// - **regex 动态替换模式**（模式 4，`use_regex=true` 且 `replace_pattern=None`
    ///   且 `regex_replace_values=None` 且 `dynamic_replace=Some`）：
    ///   匹配后保留指定捕获组内容（语义要求保留的部分：函数名/数组变量名/
    ///   prompt 文本），其余部分用字面量 + 空格填充至匹配长度，等长自动保证。
    ///   用于 AntiPromptBias / AntiAtis 等跨版本 patch 点（minified 变量名
    ///   长度跨版本变化导致匹配长度不固定，固定字面量 replace 无法满足等长）。
    ///   详见 `dynamic_replace` 字段。
    pub replace_pattern: Option<Cow<'static, [u8]>>,
    /// 内存补丁：单个字节替换
    pub patch_byte: Option<u8>,
    /// 内存补丁：替换位置偏移
    pub patch_offset: Option<usize>,
    /// 描述
    pub description: Cow<'static, str>,
    /// 是否将 search_pattern 作为 regex 处理
    ///
    /// 四种模式（见 `replace_pattern` 字段文档）：
    /// - `false`：字面量模式，`replace_pattern` 必须提供（等长字面量覆盖）。
    /// - `true` + `replace_pattern=None`：regex 数字替换模式，由
    ///   `regex_replace_values` 顺序替换匹配文本中的数字。
    /// - `true` + `replace_pattern=Some`：regex 字面量替换模式，regex 匹配后
    ///   用 `replace_pattern` 整段覆盖（等长，跨版本 patch 点用）。
    /// - `true` + `replace_pattern=None` + `regex_replace_values=None` +
    ///   `dynamic_replace=Some`：regex 动态替换模式（模式 4），匹配后保留
    ///   指定捕获组内容，其余用字面量 + 空格填充至等长（跨版本自适应）。
    pub use_regex: bool,
    /// regex 模式下的顺序替换值
    ///
    /// 例如匹配到 `var X=200000,Y=200000,...` 后，
    /// `regex_replace_values=Some(vec![500000, 500000])` 会把
    /// 第一个 200000 替换为 500000，第二个 200000 替换为 500000。
    /// 仅在 `use_regex=true` 且 `replace_pattern=None` 时生效。
    pub regex_replace_values: Option<Vec<u32>>,
    /// regex 动态替换模式（模式 4）
    ///
    /// 当 `use_regex=true` 且 `replace_pattern=None` 且 `regex_replace_values=None`
    /// 且 `dynamic_replace=Some` 时生效。
    ///
    /// 匹配后保留指定捕获组内容，其余部分用字面量 + 空格填充至等长。
    /// 用于 AntiPromptBias / AntiAtis 等跨版本 patch 点（minified 变量名长度变化）。
    pub dynamic_replace: Option<DynamicReplace>,
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
    fn test_antitelemetry_description() {
        assert!(FeatureType::AntiTelemetry
            .description()
            .contains("AntiTelemetry"));
    }

    #[test]
    fn test_antitelemetry_short_name() {
        assert_eq!(FeatureType::AntiTelemetry.short_name(), "antitelemetry");
    }

    #[test]
    fn test_antitelemetry_distinct_from_max_context_tokens() {
        assert_ne!(
            FeatureType::AntiTelemetry,
            FeatureType::MaxContextTokens
        );
    }

    #[test]
    fn test_antispy_description() {
        assert!(FeatureType::AntiSpy.description().contains("AntiSpy"));
    }

    #[test]
    fn test_antispy_short_name() {
        assert_eq!(FeatureType::AntiSpy.short_name(), "antispy");
    }

    #[test]
    fn test_antispy_distinct_from_others() {
        assert_ne!(FeatureType::AntiSpy, FeatureType::MaxContextTokens);
        assert_ne!(FeatureType::AntiSpy, FeatureType::AntiTelemetry);
    }

    #[test]
    fn test_antipromptbias_description() {
        assert!(FeatureType::AntiPromptBias
            .description()
            .contains("AntiPromptBias"));
    }

    #[test]
    fn test_antipromptbias_short_name() {
        assert_eq!(FeatureType::AntiPromptBias.short_name(), "antipromptbias");
    }

    #[test]
    fn test_antipromptbias_distinct_from_others() {
        assert_ne!(FeatureType::AntiPromptBias, FeatureType::MaxContextTokens);
        assert_ne!(FeatureType::AntiPromptBias, FeatureType::AntiTelemetry);
        assert_ne!(FeatureType::AntiPromptBias, FeatureType::AntiSpy);
    }

    #[test]
    fn test_antiatis_description() {
        assert!(FeatureType::AntiAtis.description().contains("AntiAtis"));
    }

    #[test]
    fn test_antiatis_short_name() {
        assert_eq!(FeatureType::AntiAtis.short_name(), "antiatis");
    }

    #[test]
    fn test_antiatis_distinct_from_others() {
        assert_ne!(FeatureType::AntiAtis, FeatureType::MaxContextTokens);
        assert_ne!(FeatureType::AntiAtis, FeatureType::AntiTelemetry);
        assert_ne!(FeatureType::AntiAtis, FeatureType::AntiSpy);
        assert_ne!(FeatureType::AntiAtis, FeatureType::AntiPromptBias);
    }

    #[test]
    fn test_antiframetrack_description() {
        assert!(FeatureType::AntiFrameTrack
            .description()
            .contains("AntiFrameTrack"));
    }

    #[test]
    fn test_antiframetrack_short_name() {
        assert_eq!(FeatureType::AntiFrameTrack.short_name(), "antiframetrack");
    }

    #[test]
    fn test_antiframetrack_distinct_from_others() {
        assert_ne!(FeatureType::AntiFrameTrack, FeatureType::MaxContextTokens);
        assert_ne!(FeatureType::AntiFrameTrack, FeatureType::AntiTelemetry);
        assert_ne!(FeatureType::AntiFrameTrack, FeatureType::AntiSpy);
        assert_ne!(FeatureType::AntiFrameTrack, FeatureType::AntiPromptBias);
        assert_ne!(FeatureType::AntiFrameTrack, FeatureType::AntiAtis);
    }

    #[test]
    fn test_anticloudetect_description() {
        assert!(FeatureType::AntiCloudDetect
            .description()
            .contains("AntiCloudDetect"));
    }

    #[test]
    fn test_anticloudetect_short_name() {
        assert_eq!(FeatureType::AntiCloudDetect.short_name(), "anticloudetect");
    }

    #[test]
    fn test_anticloudetect_distinct_from_others() {
        assert_ne!(FeatureType::AntiCloudDetect, FeatureType::MaxContextTokens);
        assert_ne!(FeatureType::AntiCloudDetect, FeatureType::AntiTelemetry);
        assert_ne!(FeatureType::AntiCloudDetect, FeatureType::AntiSpy);
        assert_ne!(FeatureType::AntiCloudDetect, FeatureType::AntiPromptBias);
        assert_ne!(FeatureType::AntiCloudDetect, FeatureType::AntiAtis);
        assert_ne!(FeatureType::AntiCloudDetect, FeatureType::AntiFrameTrack);
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
            dynamic_replace: None,
        };
        assert_eq!(pattern.search_pattern.as_ref(), b"var YOt=200000");
        assert!(!pattern.use_regex);
    }

    #[test]
    fn test_grok_anti_repo_bundle_variant() {
        assert_eq!(FeatureType::GrokAntiRepoBundle.short_name(), "grokantirepobundle");
        assert!(FeatureType::GrokAntiRepoBundle
            .description()
            .contains("GrokAntiRepoBundle"));
    }

    #[test]
    fn test_grok_variants_distinct() {
        assert_ne!(FeatureType::GrokAntiRepoBundle, FeatureType::GrokAntiDeployUpload);
        assert_ne!(FeatureType::GrokAntiRepoBundle, FeatureType::GrokAntiTraceUpload);
        assert_ne!(FeatureType::GrokAntiDeployUpload, FeatureType::GrokAntiTraceUpload);
    }
}
