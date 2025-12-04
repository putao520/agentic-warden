//! Integration tests for MCP intelligent routing workflows.
//!
//! Tests the complete routing pipeline from request to tool selection.

use aiw::mcp_routing::models::{DecisionMode, ExecutionMode, IntelligentRouteRequest};
use aiw::mcp_routing::registry::{DynamicToolRegistry, RegisteredTool, RegistryConfig};
use rmcp::model::Tool;
use std::sync::Arc;
use std::time::Duration;

fn build_tool(name: &str, description: &str) -> Tool {
    let mut schema = serde_json::Map::new();
    schema.insert("type".into(), serde_json::Value::String("object".into()));

    Tool {
        name: name.to_string().into(),
        title: None,
        description: Some(description.to_string().into()),
        input_schema: Arc::new(schema),
        output_schema: None,
        icons: None,
        annotations: None,
    }
}

#[tokio::test]
async fn test_dynamic_tool_registry_integration() {
    let registry = DynamicToolRegistry::new(vec![]);

    let filesystem_tools = vec![
        ("read_file", "Read file contents"),
        ("write_file", "Write to a file"),
        ("list_directory", "List directory contents"),
    ];

    for (name, desc) in filesystem_tools {
        let tool = build_tool(name, desc);
        let is_new = registry
            .register_proxied_tool("filesystem".to_string(), name.to_string(), tool)
            .await
            .unwrap();
        assert!(is_new, "First registration should be new");
    }

    let tools = registry.get_all_tool_definitions().await;
    assert_eq!(tools.len(), 3);

    let tool = registry.get_tool("read_file").await.expect("tool missing");
    match tool {
        RegisteredTool::ProxiedMcp(proxy) => assert_eq!(proxy.server, "filesystem"),
        _ => panic!("unexpected tool type"),
    }
}

#[tokio::test]
async fn test_execution_mode_values() {
    // Test that execution modes have correct default
    let default_mode = ExecutionMode::default();
    assert_eq!(default_mode, ExecutionMode::Dynamic);

    // Test mode serialization
    let dynamic_json = serde_json::to_string(&ExecutionMode::Dynamic).unwrap();
    assert_eq!(dynamic_json, "\"dynamic\"");

    let query_json = serde_json::to_string(&ExecutionMode::Query).unwrap();
    assert_eq!(query_json, "\"query\"");
}

#[tokio::test]
async fn test_decision_mode_values() {
    // Test that decision modes have correct default
    let default_mode = DecisionMode::default();
    assert_eq!(default_mode, DecisionMode::Auto);

    // Test mode serialization
    let auto_json = serde_json::to_string(&DecisionMode::Auto).unwrap();
    assert_eq!(auto_json, "\"auto\"");

    let llm_json = serde_json::to_string(&DecisionMode::LlmReact).unwrap();
    assert_eq!(llm_json, "\"llmreact\"");

    let vector_json = serde_json::to_string(&DecisionMode::Vector).unwrap();
    assert_eq!(vector_json, "\"vector\"");
}

#[tokio::test]
async fn test_routing_request_construction() {
    // Test default routing request
    let default_request = IntelligentRouteRequest::default();
    assert_eq!(default_request.execution_mode, ExecutionMode::Dynamic);
    assert_eq!(default_request.decision_mode, DecisionMode::Auto);
    assert!(default_request.user_request.is_empty());

    // Test custom routing request
    let custom_request = IntelligentRouteRequest {
        user_request: "list files in /tmp".to_string(),
        session_id: Some("test-session".to_string()),
        max_candidates: Some(5),
        decision_mode: DecisionMode::LlmReact,
        execution_mode: ExecutionMode::Query,
        metadata: [("key".to_string(), "value".to_string())]
            .iter()
            .cloned()
            .collect(),
    };

    assert_eq!(custom_request.user_request, "list files in /tmp");
    assert_eq!(custom_request.execution_mode, ExecutionMode::Query);
    assert_eq!(custom_request.decision_mode, DecisionMode::LlmReact);
    assert_eq!(custom_request.max_candidates, Some(5));
}

#[tokio::test]
async fn test_concurrent_tool_operations() {
    let registry: Arc<DynamicToolRegistry> = Arc::new(DynamicToolRegistry::new(vec![]));
    let registry_clone: Arc<DynamicToolRegistry> = Arc::clone(&registry);

    // Spawn concurrent registration tasks
    let handle1 = {
        let registry = Arc::clone(&registry);
        tokio::spawn(async move {
            for i in 0..10 {
                let tool = build_tool(&format!("tool_{}", i), &format!("Tool {}", i));
                registry
                    .register_proxied_tool("server1".to_string(), format!("tool_{}", i), tool)
                    .await
                    .unwrap();
            }
        })
    };

    let handle2 = tokio::spawn(async move {
        for i in 10..20 {
            let tool = build_tool(&format!("tool_{}", i), &format!("Tool {}", i));
            registry_clone
                .register_proxied_tool("server2".to_string(), format!("tool_{}", i), tool)
                .await
                .unwrap();
        }
    });

    // Wait for both to complete
    let _ = tokio::join!(handle1, handle2);

    // Verify all tools registered
    let tools = registry.get_all_tool_definitions().await;
    assert_eq!(tools.len(), 20);
}

#[tokio::test]
async fn test_tool_lifecycle() {
    let registry = DynamicToolRegistry::with_config(
        vec![],
        RegistryConfig {
            default_ttl_seconds: 1,
            ..RegistryConfig::default()
        },
    );

    assert!(!registry.has_tool("read_file").await);

    let tool = build_tool("read_file", "Read a file");
    let is_new = registry
        .register_proxied_tool("filesystem".to_string(), "read_file".to_string(), tool)
        .await
        .unwrap();
    assert!(is_new);
    assert!(registry.has_tool("read_file").await);

    let tool = build_tool("read_file", "Read a file (updated)");
    let is_new = registry
        .register_proxied_tool("filesystem".to_string(), "read_file".to_string(), tool)
        .await
        .unwrap();
    assert!(!is_new);

    tokio::time::sleep(Duration::from_secs(2)).await;
    registry.cleanup_expired_tools().await;
    assert!(!registry.has_tool("read_file").await);
}

#[tokio::test]
async fn test_multi_server_routing() {
    let registry = DynamicToolRegistry::new(vec![]);

    // Register tools from multiple servers
    let servers = vec![
        ("filesystem", vec!["read_file", "write_file"]),
        ("git", vec!["commit", "push", "pull"]),
        ("search", vec!["web_search", "doc_search"]),
    ];

    for (server, tools) in servers {
        for tool in tools {
            let definition = build_tool(tool, &format!("{} from {}", tool, server));
            registry
                .register_proxied_tool(server.to_string(), tool.to_string(), definition)
                .await
                .unwrap();
        }
    }

    // Verify total tool count
    assert_eq!(registry.get_all_tool_definitions().await.len(), 7);

    // Verify correct server routing
    let server = registry
        .get_tool("read_file")
        .await
        .and_then(|tool| match tool {
            RegisteredTool::ProxiedMcp(proxy) => Some(proxy.server),
            _ => None,
        })
        .unwrap();
    assert_eq!(server, "filesystem");

    let server = registry
        .get_tool("commit")
        .await
        .and_then(|tool| match tool {
            RegisteredTool::ProxiedMcp(proxy) => Some(proxy.server),
            _ => None,
        })
        .unwrap();
    assert_eq!(server, "git");

    let server = registry
        .get_tool("web_search")
        .await
        .and_then(|tool| match tool {
            RegisteredTool::ProxiedMcp(proxy) => Some(proxy.server),
            _ => None,
        })
        .unwrap();
    assert_eq!(server, "search");
}
