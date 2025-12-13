//! DynamicToolRegistry 单元测试
//!
//! 注意：这是单元测试，测试 DynamicToolRegistry 组件本身
//! 不涉及真实 MCP 服务器连接
//!
//! 真正的 E2E 测试在：
//! - tests/real_req013_phase1_capability_e2e.rs
//! - tests/real_req013_phase2_dynamic_tool_e2e.rs

use rmcp::model::Tool;
use serde_json::Map;
use std::borrow::Cow;
use std::sync::Arc;

#[tokio::test]
async fn test_dynamic_registry_base_tools() {
    use aiw::mcp_routing::registry::{DynamicToolRegistry, RegistryConfig};

    let base_tools = vec![Tool {
        name: Cow::Borrowed("intelligent_route"),
        title: Some("Router".to_string()),
        description: Some(Cow::Borrowed(
            "I can route your requests to 2 servers with 10 tools.",
        )),
        input_schema: Arc::new(Map::new()),
        output_schema: None,
        icons: None,
        annotations: None,
    }];

    let config = RegistryConfig {
        max_dynamic_tools: 5,
        default_ttl_seconds: 86400,
        cleanup_interval_seconds: 3600,
    };

    let registry = DynamicToolRegistry::with_config(base_tools, config);

    // Verify base tool is present
    let all_tools = registry.get_all_tool_definitions().await;
    assert_eq!(all_tools.len(), 1);
    assert_eq!(all_tools[0].name.as_ref(), "intelligent_route");
    assert!(all_tools[0]
        .description
        .as_ref()
        .unwrap()
        .contains("2 servers"));
}

#[tokio::test]
async fn test_dynamic_registry_fifo_eviction() {
    use aiw::mcp_routing::registry::{DynamicToolRegistry, RegistryConfig};
    use serde_json::json;

    let registry = DynamicToolRegistry::with_config(
        vec![],
        RegistryConfig {
            max_dynamic_tools: 5,
            default_ttl_seconds: 86400,
            cleanup_interval_seconds: 3600,
        },
    );

    // Register 6 dynamic tools (should evict the first one - FIFO)
    for i in 0..6 {
        registry
            .register_js_tool(
                format!("workflow_{}", i),
                format!("Workflow {}", i),
                json!({"type": "object"}),
                format!("async function workflow_{}() {{}}", i),
            )
            .await
            .unwrap();
    }

    // Verify only 5 tools remain
    let all_tools = registry.get_all_tool_definitions().await;
    assert_eq!(all_tools.len(), 5, "Expected 5 tools after FIFO eviction");

    // Verify the first tool (workflow_0) was evicted
    let has_workflow_0 = all_tools.iter().any(|t| t.name.as_ref() == "workflow_0");
    assert!(!has_workflow_0, "workflow_0 should have been evicted");

    // Verify tools 1-5 are present
    for i in 1..=5 {
        let has_tool = all_tools
            .iter()
            .any(|t| t.name.as_ref() == format!("workflow_{}", i));
        assert!(has_tool, "workflow_{} should be present", i);
    }
}

#[tokio::test]
async fn test_dynamic_registry_mixed_tools() {
    use aiw::mcp_routing::registry::{DynamicToolRegistry, RegistryConfig};
    use serde_json::json;

    let base_tools = vec![Tool {
        name: Cow::Borrowed("intelligent_route"),
        description: Some(Cow::Borrowed("Router tool")),
        input_schema: Arc::new(Map::new()),
        title: None,
        output_schema: None,
        icons: None,
        annotations: None,
    }];

    let registry = DynamicToolRegistry::with_config(
        base_tools,
        RegistryConfig {
            max_dynamic_tools: 3,
            default_ttl_seconds: 86400,
            cleanup_interval_seconds: 3600,
        },
    );

    // Register 2 dynamic tools
    registry
        .register_js_tool(
            "dynamic_1".to_string(),
            "Dynamic tool 1".to_string(),
            json!({"type": "object"}),
            "async function() {}".to_string(),
        )
        .await
        .unwrap();

    registry
        .register_js_tool(
            "dynamic_2".to_string(),
            "Dynamic tool 2".to_string(),
            json!({"type": "object"}),
            "async function() {}".to_string(),
        )
        .await
        .unwrap();

    // Verify total count: 1 base + 2 dynamic = 3
    let all_tools = registry.get_all_tool_definitions().await;
    assert_eq!(all_tools.len(), 3);

    // Verify base tool is present
    let has_base = all_tools
        .iter()
        .any(|t| t.name.as_ref() == "intelligent_route");
    assert!(has_base, "Base tool should be present");

    // Verify dynamic tools are present
    let has_dynamic_1 = all_tools.iter().any(|t| t.name.as_ref() == "dynamic_1");
    let has_dynamic_2 = all_tools.iter().any(|t| t.name.as_ref() == "dynamic_2");
    assert!(has_dynamic_1, "dynamic_1 should be present");
    assert!(has_dynamic_2, "dynamic_2 should be present");
}

#[tokio::test]
async fn test_tool_registry_has_tool() {
    use aiw::mcp_routing::registry::DynamicToolRegistry;
    use serde_json::json;

    let base_tools = vec![Tool {
        name: Cow::Borrowed("base_tool"),
        description: Some(Cow::Borrowed("Base")),
        input_schema: Arc::new(Map::new()),
        title: None,
        output_schema: None,
        icons: None,
        annotations: None,
    }];

    let registry = DynamicToolRegistry::new(base_tools);

    // Check base tool
    assert!(registry.has_tool("base_tool").await);

    // Register dynamic tool
    registry
        .register_js_tool(
            "dynamic_tool".to_string(),
            "Dynamic".to_string(),
            json!({"type": "object"}),
            "async function() {}".to_string(),
        )
        .await
        .unwrap();

    // Check dynamic tool
    assert!(registry.has_tool("dynamic_tool").await);

    // Check non-existent tool
    assert!(!registry.has_tool("non_existent").await);
}

// Note: Full integration test with IntelligentRouter::initialize() would require:
// 1. Running MCP servers (filesystem, memory, etc.)
// 2. Valid Ollama endpoint for LLM mode
// 3. Proper environment configuration
//
// These tests focus on the core components in isolation.
// For full E2E testing with real MCP servers, use docker-compose.ci.yml environment.
