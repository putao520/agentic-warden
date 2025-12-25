//! MCP plugin filtering logic.

use crate::commands::market::plugin::PluginMetadata;

pub trait McpFilter {
    fn is_mcp_plugin(plugin: &PluginMetadata) -> bool;
    fn filter_mcp_plugins(plugins: Vec<PluginMetadata>) -> Vec<PluginMetadata>;
}

impl McpFilter for PluginMetadata {
    fn is_mcp_plugin(plugin: &PluginMetadata) -> bool {
        plugin.has_mcp_servers
    }

    fn filter_mcp_plugins(plugins: Vec<PluginMetadata>) -> Vec<PluginMetadata> {
        plugins
            .into_iter()
            .filter(|plugin| Self::is_mcp_plugin(plugin))
            .collect()
    }
}
