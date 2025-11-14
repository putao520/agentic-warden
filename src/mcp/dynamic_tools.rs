//! Dynamic tool registration manager.
//!
//! Manages tools that are registered on-demand based on routing decisions.

use rmcp::model::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages dynamically registered MCP tools.
#[derive(Clone)]
pub struct DynamicToolManager {
    /// Currently registered tools: tool_name -> (mcp_server, tool)
    tools: Arc<RwLock<HashMap<String, (String, Tool)>>>,
}

impl DynamicToolManager {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new tool dynamically.
    /// Returns true if this is a new registration (tool list changed).
    pub async fn register_tool(
        &self,
        mcp_server: String,
        tool_name: String,
        description: String,
        input_schema: serde_json::Value,
    ) -> bool {
        // Convert serde_json::Value to JsonObject (Map<String, Value>)
        let schema_object = match input_schema {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(), // Fallback to empty object if not an object
        };

        let tool = Tool {
            name: tool_name.clone().into(),
            description: Some(description.into()),
            input_schema: Arc::new(schema_object),
            output_schema: None,
            annotations: None,
        };

        let mut tools = self.tools.write().await;
        let is_new = !tools.contains_key(&tool_name);
        tools.insert(tool_name, (mcp_server, tool));

        is_new
    }

    /// Remove a dynamically registered tool.
    pub async fn unregister_tool(&self, tool_name: &str) -> bool {
        self.tools.write().await.remove(tool_name).is_some()
    }

    /// Get all currently registered dynamic tools.
    pub async fn list_tools(&self) -> Vec<Tool> {
        self.tools
            .read()
            .await
            .values()
            .map(|(_, tool)| tool.clone())
            .collect()
    }

    /// Get the MCP server for a specific tool.
    pub async fn get_server(&self, tool_name: &str) -> Option<String> {
        self.tools
            .read()
            .await
            .get(tool_name)
            .map(|(server, _)| server.clone())
    }

    /// Clear all dynamically registered tools.
    pub async fn clear(&self) {
        self.tools.write().await.clear();
    }

    /// Check if a tool is currently registered.
    pub async fn has_tool(&self, tool_name: &str) -> bool {
        self.tools.read().await.contains_key(tool_name)
    }
}

impl Default for DynamicToolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_list() {
        let manager = DynamicToolManager::new();

        let is_new = manager
            .register_tool(
                "filesystem".to_string(),
                "read_file".to_string(),
                "Read a file".to_string(),
                serde_json::json!({"type": "object"}),
            )
            .await;

        assert!(is_new);

        let tools = manager.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read_file");
    }

    #[tokio::test]
    async fn test_unregister() {
        let manager = DynamicToolManager::new();

        manager
            .register_tool(
                "filesystem".to_string(),
                "read_file".to_string(),
                "Read a file".to_string(),
                serde_json::json!({"type": "object"}),
            )
            .await;

        let removed = manager.unregister_tool("read_file").await;
        assert!(removed);

        let tools = manager.list_tools().await;
        assert_eq!(tools.len(), 0);
    }

    #[tokio::test]
    async fn test_get_server() {
        let manager = DynamicToolManager::new();

        manager
            .register_tool(
                "filesystem".to_string(),
                "read_file".to_string(),
                "Read a file".to_string(),
                serde_json::json!({"type": "object"}),
            )
            .await;

        let server = manager.get_server("read_file").await;
        assert_eq!(server, Some("filesystem".to_string()));

        let missing = manager.get_server("nonexistent").await;
        assert_eq!(missing, None);
    }
}
