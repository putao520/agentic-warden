//! Claude CLI 专有 patch 模块（7 个 feature + 版本工具 + 安装探测）

pub mod install;
pub mod registry;
pub mod versions;

pub use registry::{
    get_anticloudetect_patches, get_antiframetrack_patches, get_antiatis_patches,
    get_antipromptbias_patches, get_antispy_patches, get_antitelemetry_patches,
    get_feature_patches, get_max_context_tokens_patches,
};
pub use versions::{ClaudeVersion, MAX_CONTEXT_TOKENS_SEARCH_REGEX, encode_max_context_tokens, validate_max_context_tokens};
pub use install::{detect_installation, get_claude_cli_path, get_claude_js_path, get_patchable_path, is_npm_installation, InstallationType};
