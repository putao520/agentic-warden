//! 配置路径管理
//!
//! 管理所有配置文件的路径
//!
//! 设计原则:
//! - 持久化配置（provider、auth、config）保存在 ~/.aiw/
//! - 运行时数据（日志、临时文件）保存在 /tmp/.aiw/

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 用户配置（从 config.json 读取）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    /// 用户角色目录（默认 ~/.aiw/role/）
    #[serde(default)]
    pub user_roles_dir: Option<String>,
}

impl UserConfig {
    /// 从配置文件加载
    pub fn load(config_file: &PathBuf) -> Self {
        if config_file.exists() {
            if let Ok(content) = std::fs::read_to_string(config_file) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// 获取用户角色目录（支持 ~ 展开）
    pub fn get_user_roles_dir(&self) -> Option<PathBuf> {
        self.user_roles_dir.as_ref().map(|dir| {
            if dir.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    return home.join(&dir[2..]);
                }
            }
            PathBuf::from(dir)
        })
    }
}

/// 配置文件路径集合
pub struct ConfigPaths {
    /// 持久化配置目录（~/.aiw/）
    pub config_dir: PathBuf,
    /// 运行时数据目录（/tmp/.aiw/）
    pub runtime_dir: PathBuf,
    /// Provider 配置文件
    pub provider_config: PathBuf,
    /// 认证信息文件
    pub auth_file: PathBuf,
    /// 主配置文件
    pub config_file: PathBuf,
    /// 日志文件（保存在运行时目录）
    pub log_file: PathBuf,
    /// 临时文件目录（保存在运行时目录）
    pub temp_dir: PathBuf,
    /// 用户配置
    pub user_config: UserConfig,
}

impl ConfigPaths {
    /// 创建配置路径
    pub fn new() -> Result<Self> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

        // 持久化配置目录：~/.aiw/
        let config_dir = home_dir.join(".aiw");

        // 运行时数据目录：使用系统临时目录（跨平台）
        // Linux/macOS: /tmp/.aiw/
        // Windows: %TEMP%\.aiw\
        let runtime_dir = std::env::temp_dir().join(".aiw");

        let config_file = config_dir.join("config.json");
        let user_config = UserConfig::load(&config_file);

        Ok(Self {
            provider_config: config_dir.join("provider.json"),
            auth_file: config_dir.join("auth.json"),
            config_file,
            log_file: runtime_dir.join("aiw.log"),
            temp_dir: runtime_dir.join("temp"),
            config_dir,
            runtime_dir,
            user_config,
        })
    }

    /// 确保配置目录存在
    pub fn ensure_dirs(&self) -> Result<()> {
        // 确保持久化配置目录存在
        std::fs::create_dir_all(&self.config_dir)?;
        // 确保运行时数据目录存在
        std::fs::create_dir_all(&self.runtime_dir)?;
        std::fs::create_dir_all(&self.temp_dir)?;
        Ok(())
    }
}
