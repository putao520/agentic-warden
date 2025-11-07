//! 配置路径管理
//!
//! 管理所有配置文件的路径

use anyhow::Result;
use std::path::PathBuf;

/// 配置文件路径集合
pub struct ConfigPaths {
    /// 主配置目录
    pub config_dir: PathBuf,
    /// Provider 配置文件
    pub provider_config: PathBuf,
    /// 认证信息文件
    pub auth_file: PathBuf,
    /// 主配置文件
    pub config_file: PathBuf,
    /// 日志文件
    pub log_file: PathBuf,
    /// 临时文件目录
    pub temp_dir: PathBuf,
}

impl ConfigPaths {
    /// 创建配置路径
    pub fn new() -> Result<Self> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

        let config_dir = home_dir.join(".agentic-warden");

        Ok(Self {
            provider_config: config_dir.join("provider.json"),
            auth_file: config_dir.join("auth.json"),
            config_file: config_dir.join("config.json"),
            log_file: config_dir.join("agentic-warden.log"),
            temp_dir: config_dir.join("temp"),
            config_dir,
        })
    }

    /// 确保配置目录存在
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.temp_dir)?;
        Ok(())
    }
}
