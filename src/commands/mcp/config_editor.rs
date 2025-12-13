//! MCP配置文件编辑器
//!
//! 提供对 ~/.aiw/mcp.json 的读写和操作功能

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// MCP服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// MCP配置文件根结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// MCP配置文件编辑器
pub struct McpConfigEditor {
    config_path: PathBuf,
}

impl McpConfigEditor {
    /// 创建新的配置编辑器
    pub fn new() -> Result<Self> {
        let config_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot find home directory"))?
            .join(".aiw")
            .join("mcp.json");

        Ok(Self { config_path })
    }

    /// 获取配置文件路径
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// 确保配置目录存在
    fn ensure_config_dir(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }
        Ok(())
    }

    /// 读取配置文件
    pub fn read(&self) -> Result<McpConfig> {

        if !self.config_path.exists() {
            // 配置文件不存在，返回空配置
            return Ok(McpConfig {
                mcp_servers: HashMap::new(),
            });
        }

        let content = fs::read_to_string(&self.config_path).with_context(|| {
            format!(
                "Failed to read MCP config from {}",
                self.config_path.display()
            )
        })?;

        let config: McpConfig = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in {}", self.config_path.display()))?;

        Ok(config)
    }

    /// 写入配置文件
    pub fn write(&self, config: &McpConfig) -> Result<()> {
        self.ensure_config_dir()?;

        let content =
            serde_json::to_string_pretty(config).context("Failed to serialize MCP config")?;

        fs::write(&self.config_path, content).with_context(|| {
            format!(
                "Failed to write MCP config to {}",
                self.config_path.display()
            )
        })?;

        Ok(())
    }

    /// 检查服务器是否存在
    pub fn server_exists(&self, name: &str) -> Result<bool> {
        let config = self.read()?;
        Ok(config.mcp_servers.contains_key(name))
    }

    /// 获取服务器配置
    pub fn get_server(&self, name: &str) -> Result<Option<McpServerConfig>> {
        let config = self.read()?;
        Ok(config.mcp_servers.get(name).cloned())
    }

    /// 添加服务器
    pub fn add_server(&self, name: &str, server_config: McpServerConfig) -> Result<()> {
        let mut config = self.read()?;

        if config.mcp_servers.contains_key(name) {
            return Err(anyhow!("MCP server '{}' already exists", name));
        }

        config.mcp_servers.insert(name.to_string(), server_config);
        self.write(&config)?;

        Ok(())
    }

    /// 移除服务器
    pub fn remove_server(&self, name: &str) -> Result<()> {
        let mut config = self.read()?;

        if config.mcp_servers.remove(name).is_none() {
            return Err(anyhow!("MCP server '{}' not found", name));
        }

        self.write(&config)?;

        Ok(())
    }

    /// 更新服务器配置
    pub fn update_server(&self, name: &str, server_config: McpServerConfig) -> Result<()> {
        let mut config = self.read()?;

        if !config.mcp_servers.contains_key(name) {
            return Err(anyhow!("MCP server '{}' not found", name));
        }

        config.mcp_servers.insert(name.to_string(), server_config);
        self.write(&config)?;

        Ok(())
    }

    /// 设置服务器启用状态
    pub fn set_server_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let mut config = self.read()?;

        let server = config
            .mcp_servers
            .get_mut(name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", name))?;

        server.enabled = Some(enabled);
        self.write(&config)?;

        Ok(())
    }

    /// 列出所有服务器
    pub fn list_servers(&self) -> Result<Vec<(String, McpServerConfig)>> {
        let config = self.read()?;

        let mut servers: Vec<(String, McpServerConfig)> = config.mcp_servers.into_iter().collect();

        // 按名称排序
        servers.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(servers)
    }

    /// 获取服务器数量统计
    pub fn server_stats(&self) -> Result<(usize, usize, usize)> {
        let servers = self.list_servers()?;
        let total = servers.len();
        let enabled = servers
            .iter()
            .filter(|(_, cfg)| cfg.enabled.unwrap_or(true))
            .count();
        let disabled = total - enabled;

        Ok((total, enabled, disabled))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, McpConfigEditor) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("mcp.json");

        // 临时修改HOME环境变量
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let editor = McpConfigEditor {
            config_path: temp_dir.path().join(".aiw").join("mcp.json"),
        };

        // 恢复原始HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        }

        (temp_dir, editor)
    }

    #[test]
    fn test_read_empty_config() {
        let (_temp, editor) = setup_test_env();
        let config = editor.read().unwrap();
        assert_eq!(config.mcp_servers.len(), 0);
    }

    #[test]
    fn test_add_server() {
        let (_temp, editor) = setup_test_env();

        let server_config = McpServerConfig {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "test-server".to_string()],
            env: HashMap::new(),
            description: Some("Test server".to_string()),
            category: Some("test".to_string()),
            enabled: Some(true),
            source: None,
        };

        editor.add_server("test", server_config).unwrap();

        let config = editor.read().unwrap();
        assert_eq!(config.mcp_servers.len(), 1);
        assert!(config.mcp_servers.contains_key("test"));
    }

    #[test]
    fn test_remove_server() {
        let (_temp, editor) = setup_test_env();

        let server_config = McpServerConfig {
            command: "npx".to_string(),
            args: vec![],
            env: HashMap::new(),
            description: None,
            category: None,
            enabled: Some(true),
            source: None,
        };

        editor.add_server("test", server_config).unwrap();
        editor.remove_server("test").unwrap();

        let config = editor.read().unwrap();
        assert_eq!(config.mcp_servers.len(), 0);
    }

    #[test]
    fn test_duplicate_server() {
        let (_temp, editor) = setup_test_env();

        let server_config = McpServerConfig {
            command: "npx".to_string(),
            args: vec![],
            env: HashMap::new(),
            description: None,
            category: None,
            enabled: Some(true),
            source: None,
        };

        editor.add_server("test", server_config.clone()).unwrap();
        let result = editor.add_server("test", server_config);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_set_enabled() {
        let (_temp, editor) = setup_test_env();

        let server_config = McpServerConfig {
            command: "npx".to_string(),
            args: vec![],
            env: HashMap::new(),
            description: None,
            category: None,
            enabled: Some(true),
            source: None,
        };

        editor.add_server("test", server_config).unwrap();
        editor.set_server_enabled("test", false).unwrap();

        let server = editor.get_server("test").unwrap().unwrap();
        assert_eq!(server.enabled, Some(false));
    }
}
