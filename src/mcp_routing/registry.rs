//! Dynamic Tool Registry (REQ-013)
//!
//! Acts as the single source of truth for all MCP tool definitions.
//! Maintains base tools and TTL-scoped dynamic tools with eviction.

use anyhow::{anyhow, Result};
use rmcp::model::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Registry configuration (defaults follow SPEC/02-ARCHITECTURE.md §1157-1201)
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Default TTL for dynamic tools (seconds)
    pub default_ttl_seconds: u64,
    /// Maximum number of dynamic tools retained simultaneously
    pub max_dynamic_tools: usize,
    /// Background cleanup interval (seconds)
    pub cleanup_interval_seconds: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            default_ttl_seconds: 120, // 2 minutes TTL for dynamic tools
            max_dynamic_tools: 100,
            cleanup_interval_seconds: 60,
        }
    }
}

/// Classifies a dynamic tool (JS orchestration vs proxied MCP)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicToolType {
    JsOrchestrated,
    ProxiedMcp,
}

/// Metadata tracked for every dynamic tool entry
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub registered_at: Instant,
    pub ttl_seconds: u64,
    pub execution_count: u64,
}

impl ToolMetadata {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            registered_at: Instant::now(),
            ttl_seconds,
            execution_count: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.registered_at.elapsed().as_secs() >= self.ttl_seconds
    }

    pub fn record_execution(&mut self) {
        self.execution_count = self.execution_count.saturating_add(1);
    }
}

/// JS orchestrated dynamic tool definition
#[derive(Debug, Clone)]
pub struct JsOrchestratedTool {
    pub tool: Tool,
    pub js_code: String,
    pub metadata: ToolMetadata,
}

/// Proxied MCP tool definition
#[derive(Debug, Clone)]
pub struct ProxiedMcpTool {
    pub tool: Tool,
    pub server: String,
    pub original_name: String,
    pub metadata: ToolMetadata,
}

/// Registered tool entry within the registry map
#[derive(Debug, Clone)]
pub enum RegisteredTool {
    JsOrchestrated(JsOrchestratedTool),
    ProxiedMcp(ProxiedMcpTool),
}

impl RegisteredTool {
    fn metadata(&self) -> &ToolMetadata {
        match self {
            RegisteredTool::JsOrchestrated(tool) => &tool.metadata,
            RegisteredTool::ProxiedMcp(tool) => &tool.metadata,
        }
    }

    fn metadata_mut(&mut self) -> &mut ToolMetadata {
        match self {
            RegisteredTool::JsOrchestrated(tool) => &mut tool.metadata,
            RegisteredTool::ProxiedMcp(tool) => &mut tool.metadata,
        }
    }

    pub fn tool(&self) -> &Tool {
        match self {
            RegisteredTool::JsOrchestrated(tool) => &tool.tool,
            RegisteredTool::ProxiedMcp(tool) => &tool.tool,
        }
    }

    pub fn tool_type(&self) -> DynamicToolType {
        match self {
            RegisteredTool::JsOrchestrated(_) => DynamicToolType::JsOrchestrated,
            RegisteredTool::ProxiedMcp(_) => DynamicToolType::ProxiedMcp,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.metadata().is_expired()
    }

    fn record_execution(&mut self) -> u64 {
        let meta = self.metadata_mut();
        meta.record_execution();
        meta.execution_count
    }
}

impl RegisteredTool {
    fn new_js(tool: Tool, js_code: String, ttl: u64) -> Self {
        RegisteredTool::JsOrchestrated(JsOrchestratedTool {
            tool,
            js_code,
            metadata: ToolMetadata::new(ttl),
        })
    }

    fn new_proxied(tool: Tool, server: String, original_name: String, ttl: u64) -> Self {
        RegisteredTool::ProxiedMcp(ProxiedMcpTool {
            tool,
            server,
            original_name,
            metadata: ToolMetadata::new(ttl),
        })
    }

    fn registered_at(&self) -> Instant {
        self.metadata().registered_at
    }
}

/// Convenience wrapper for registering batches of proxied tools
#[derive(Debug, Clone)]
pub struct ProxiedToolRegistration {
    pub original_name: String,
    pub tool: Tool,
}

impl ProxiedToolRegistration {
    pub fn new(tool: Tool) -> Self {
        let original_name = tool.name.to_string();
        Self {
            original_name,
            tool,
        }
    }
}

/// Base tool definition (permanent entry)
#[derive(Debug, Clone)]
pub struct BaseToolDefinition {
    pub tool: Tool,
}

/// Dynamic tool registry implementation (thread-safe)
pub struct DynamicToolRegistry {
    base_tools: Arc<RwLock<HashMap<String, BaseToolDefinition>>>,
    base_snapshot: Arc<RwLock<Vec<Tool>>>,
    dynamic_tools: Arc<RwLock<HashMap<String, RegisteredTool>>>,
    config: RegistryConfig,
    tool_cache: Arc<RwLock<Option<Arc<Vec<Tool>>>>>,
}

impl DynamicToolRegistry {
    /// Create a new registry with default configuration
    pub fn new(base_tools: Vec<Tool>) -> Self {
        Self::with_config(base_tools, RegistryConfig::default())
    }

    /// Create a registry with custom configuration (used for testing/tuning)
    pub fn with_config(base_tools: Vec<Tool>, config: RegistryConfig) -> Self {
        let mut base_map = HashMap::new();
        let mut base_snapshot = Vec::with_capacity(base_tools.len());
        for tool in base_tools {
            base_snapshot.push(tool.clone());
            base_map.insert(tool.name.to_string(), BaseToolDefinition { tool });
        }

        Self {
            base_tools: Arc::new(RwLock::new(base_map)),
            base_snapshot: Arc::new(RwLock::new(base_snapshot)),
            dynamic_tools: Arc::new(RwLock::new(HashMap::new())),
            config,
            tool_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Extend base tools with additional tools (used to merge server tools)
    pub async fn extend_base_tools(&self, tools: Vec<Tool>) {
        let mut base_map = self.base_tools.write().await;
        let mut base_snapshot = self.base_snapshot.write().await;

        for tool in tools {
            if !base_map.contains_key(tool.name.as_ref()) {
                base_snapshot.push(tool.clone());
                base_map.insert(tool.name.to_string(), BaseToolDefinition { tool });
            }
        }

        // Invalidate cache since base tools changed
        drop(base_map);
        drop(base_snapshot);
        self.invalidate_cache().await;
    }

    /// Start the background cleanup loop, returning the JoinHandle for the caller to manage
    pub fn start_cleanup_task(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let registry = Arc::clone(self);
        let interval_secs = self.config.cleanup_interval_seconds.max(1);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                let removed = registry.cleanup_expired_tools().await;
                if removed > 0 {
                    eprintln!("🧹 Cleaned up {removed} expired dynamic tools");
                }
            }
        })
    }

    /// Register a JS orchestrated tool (LLM generated workflow)
    pub async fn register_js_tool(
        &self,
        name: String,
        description: String,
        input_schema: serde_json::Value,
        js_code: String,
    ) -> Result<bool> {
        if name.trim().is_empty() {
            return Err(anyhow!("Tool name cannot be empty"));
        }

        let schema_object = match input_schema {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };

        let tool = Tool {
            name: name.clone().into(),
            title: None,
            description: Some(description.into()),
            input_schema: Arc::new(schema_object),
            output_schema: None,
            icons: None,
            annotations: None,
        };

        let mut tools = self.dynamic_tools.write().await;
        self.evict_if_needed(&mut tools);
        let is_new = !tools.contains_key(&name);
        tools.insert(
            name,
            RegisteredTool::new_js(tool, js_code, self.config.default_ttl_seconds),
        );
        drop(tools);
        self.invalidate_cache().await;

        Ok(is_new)
    }

    /// Register a single proxied MCP tool
    pub async fn register_proxied_tool(
        &self,
        server: String,
        original_name: String,
        tool: Tool,
    ) -> Result<bool> {
        if server.trim().is_empty() {
            return Err(anyhow!("Server name cannot be empty"));
        }

        let tool_name = tool.name.to_string();
        let mut tools = self.dynamic_tools.write().await;
        self.evict_if_needed(&mut tools);
        let is_new = !tools.contains_key(&tool_name);
        tools.insert(
            tool_name,
            RegisteredTool::new_proxied(
                tool,
                server,
                original_name,
                self.config.default_ttl_seconds,
            ),
        );
        drop(tools);
        self.invalidate_cache().await;

        Ok(is_new)
    }

    /// Register a batch of proxied tools (returns count of new registrations)
    pub async fn register_proxied_tools(
        &self,
        server: String,
        definitions: Vec<ProxiedToolRegistration>,
    ) -> Result<usize> {
        let mut new_count = 0;
        for definition in definitions {
            if self
                .register_proxied_tool(server.clone(), definition.original_name, definition.tool)
                .await?
            {
                new_count += 1;
            }
        }
        Ok(new_count)
    }

    /// Return base + dynamic tool definitions for list_tools
    pub async fn get_all_tool_definitions(&self) -> Arc<Vec<Tool>> {
        // Opportunistically clean up expired entries before snapshotting
        self.cleanup_expired_tools_inner().await;

        if let Some(cached) = self.tool_cache.read().await.clone() {
            return cached;
        }

        let base_tools = self.base_tools.read().await;
        let base_snapshot = self.base_snapshot.read().await;
        let mut snapshot = Vec::with_capacity(base_tools.len() + self.config.max_dynamic_tools);
        snapshot.extend(base_snapshot.iter().cloned());
        drop(base_tools);
        drop(base_snapshot);

        let map = self.dynamic_tools.read().await;
        for entry in map.values() {
            snapshot.push(entry.tool().clone());
        }
        drop(map);

        let arc_snapshot = Arc::new(snapshot);
        *self.tool_cache.write().await = Some(arc_snapshot.clone());
        arc_snapshot
    }

    /// Fetch a dynamic tool entry by name
    pub async fn get_tool(&self, name: &str) -> Option<RegisteredTool> {
        let map = self.dynamic_tools.read().await;
        map.get(name).cloned()
    }

    /// Whether a tool exists (base or dynamic)
    pub async fn has_tool(&self, name: &str) -> bool {
        if self.base_tools.read().await.contains_key(name) {
            return true;
        }
        self.dynamic_tools.read().await.contains_key(name)
    }

    /// Get the number of dynamically registered tools
    pub async fn dynamic_tool_count(&self) -> usize {
        self.dynamic_tools.read().await.len()
    }

    /// Increment execution count for a tool and return the updated number
    pub async fn record_execution(&self, name: &str) -> Option<u64> {
        let mut map = self.dynamic_tools.write().await;
        map.get_mut(name).map(|entry| entry.record_execution())
    }

    /// Manually remove a dynamic tool entry (used for cleanup/testing)
    pub async fn unregister_tool(&self, name: &str) -> bool {
        let removed = self.dynamic_tools.write().await.remove(name).is_some();
        if removed {
            self.invalidate_cache().await;
        }
        removed
    }

    /// Cleanup expired tools (returns number removed)
    pub async fn cleanup_expired_tools(&self) -> usize {
        self.cleanup_expired_tools_inner().await
    }

    async fn cleanup_expired_tools_inner(&self) -> usize {
        let mut tools = self.dynamic_tools.write().await;
        let before = tools.len();
        tools.retain(|_, tool| !tool.is_expired());
        let removed = before.saturating_sub(tools.len());
        drop(tools);
        if removed > 0 {
            self.invalidate_cache().await;
        }
        removed
    }

    async fn invalidate_cache(&self) {
        *self.tool_cache.write().await = None;
    }

    fn evict_if_needed(&self, tools: &mut HashMap<String, RegisteredTool>) {
        if tools.len() < self.config.max_dynamic_tools {
            return;
        }

        if let Some(oldest) = Self::find_oldest_tool(tools) {
            tools.remove(&oldest);
            eprintln!("⚠️  Tool limit reached, evicted oldest tool: {oldest}");
        }
    }

    fn find_oldest_tool(tools: &HashMap<String, RegisteredTool>) -> Option<String> {
        tools
            .iter()
            .min_by_key(|(_, tool)| tool.registered_at())
            .map(|(name, _)| name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tool(name: &str) -> Tool {
        Tool {
            name: name.to_string().into(),
            title: None,
            description: Some("Test tool".into()),
            input_schema: Arc::new(serde_json::Map::new()),
            output_schema: None,
            icons: None,
            annotations: None,
        }
    }

    #[tokio::test]
    async fn test_register_js_tool() {
        let registry = DynamicToolRegistry::new(vec![]);

        let result = registry
            .register_js_tool(
                "workflow".to_string(),
                "Test workflow".to_string(),
                serde_json::json!({"type": "object"}),
                "async function workflow() {}".to_string(),
            )
            .await
            .unwrap();

        assert!(result);
        assert!(registry.has_tool("workflow").await);
    }

    #[tokio::test]
    async fn test_register_proxied_tool() {
        let registry = DynamicToolRegistry::new(vec![]);
        let tool = create_test_tool("read_file");

        let is_new = registry
            .register_proxied_tool("filesystem".to_string(), "read_file".to_string(), tool)
            .await
            .unwrap();

        assert!(is_new);
        assert!(registry.has_tool("read_file").await);
    }

    #[tokio::test]
    async fn test_tool_expiration_cleanup() {
        let registry = DynamicToolRegistry::with_config(
            vec![],
            RegistryConfig {
                default_ttl_seconds: 1,
                max_dynamic_tools: 10,
                cleanup_interval_seconds: 1,
            },
        );

        registry
            .register_js_tool(
                "temp".to_string(),
                "Temp".to_string(),
                serde_json::json!({"type": "object"}),
                "async function workflow() {}".to_string(),
            )
            .await
            .unwrap();

        assert!(registry.has_tool("temp").await);
        tokio::time::sleep(Duration::from_secs(2)).await;
        let removed = registry.cleanup_expired_tools().await;
        assert_eq!(removed, 1);
        assert!(!registry.has_tool("temp").await);
    }

    #[tokio::test]
    async fn test_tool_limit_eviction() {
        let registry = DynamicToolRegistry::with_config(
            vec![],
            RegistryConfig {
                default_ttl_seconds: 100,
                max_dynamic_tools: 3,
                cleanup_interval_seconds: 60,
            },
        );

        for idx in 0..4 {
            let tool = create_test_tool(&format!("tool_{idx}"));
            registry
                .register_proxied_tool("server".to_string(), format!("tool_{idx}"), tool)
                .await
                .unwrap();
        }

        let tools = registry.get_all_tool_definitions().await;
        assert_eq!(tools.len(), 3);
    }

    #[tokio::test]
    async fn test_record_execution_counter() {
        let registry = DynamicToolRegistry::new(vec![]);
        registry
            .register_js_tool(
                "exec".to_string(),
                "Exec".to_string(),
                serde_json::json!({}),
                "async function workflow() {}".to_string(),
            )
            .await
            .unwrap();

        let count = registry.record_execution("exec").await;
        assert_eq!(count, Some(1));
        let count = registry.record_execution("exec").await;
        assert_eq!(count, Some(2));
    }
}
