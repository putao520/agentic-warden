//! Grok CLI 专有 patch 模块（3 个上传禁用 feature + 版本 + 锚点定位 + 安装探测）

pub mod install;
pub mod registry;
pub mod targets;
pub mod versions;

pub use install::{detect_grok, get_grok_binary_path, GrokInstallation};
pub use registry::{
    get_grok_deploy_upload_patches, get_grok_repo_bundle_patches, get_grok_trace_upload_patches,
};
pub use versions::GrokVersion;
