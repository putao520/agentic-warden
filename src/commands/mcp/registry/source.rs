use super::types::{McpServerDetail, McpServerInfo};
use crate::commands::mcp::McpServerConfig;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait RegistrySource: Send + Sync {
    /// Human readable source name.
    fn source_name(&self) -> &'static str;

    /// Stable identifier used for filtering (e.g., "registry", "smithery").
    fn source_id(&self) -> &'static str;

    /// Priority for deduplication (lower = higher priority).
    fn priority(&self) -> u8;

    /// Search servers by keyword.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<McpServerInfo>>;

    /// Fetch full server detail; returns None when not found in this source.
    async fn get_server(&self, name: &str) -> Result<Option<McpServerDetail>>;

    /// Build install-ready config for the given server name.
    async fn get_install_config(&self, name: &str) -> Result<McpServerConfig>;
}
