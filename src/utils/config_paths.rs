//! 配置路径管理
//!
//! 管理所有配置文件的路径
//!
//! 设计原则:
//! - 持久化配置（provider、auth、config）保存在 ~/.aiw/
//! - 运行时数据（日志、临时文件）保存在 /tmp/.aiw/

use anyhow::Result;
use std::path::PathBuf;

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

        Ok(Self {
            provider_config: config_dir.join("provider.json"),
            auth_file: config_dir.join("auth.json"),
            config_file: config_dir.join("config.json"),
            log_file: runtime_dir.join("aiw.log"),
            temp_dir: runtime_dir.join("temp"),
            config_dir,
            runtime_dir,
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
