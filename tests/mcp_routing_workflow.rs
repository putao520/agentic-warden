//! Integration tests for MCP intelligent routing workflows.
//!
//! Tests the complete routing pipeline from request to tool selection.

use agentic_warden::mcp::dynamic_tools::DynamicToolManager;
use agentic_warden::mcp_routing::models::{
    DecisionMode, ExecutionMode, IntelligentRouteRequest,
};
use serde_json::json;

#[tokio::test]
async fn test_dynamic_tool_manager_integration() {
    let manager = DynamicToolManager::new();

    // Simulate routing workflow: register filesystem tools
    let filesystem_tools = vec![
        ("read_file", "Read file contents"),
        ("write_file", "Write to a file"),
        ("list_directory", "List directory contents"),
    ];

    for (name, desc) in filesystem_tools {
        let is_new = manager
            .register_tool(
                "filesystem".to_string(),
                name.to_string(),
                desc.to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    }
                }),
            )
            .await;
        assert!(is_new, "First registration should be new");
    }

    // Verify all tools registered
    let tools = manager.list_tools().await;
    assert_eq!(tools.len(), 3);

    // Verify server routing works
    assert_eq!(
        manager.get_server("read_file").await,
        Some("filesystem".to_string())
    );
    assert_eq!(
        manager.get_server("write_file").await,
        Some("filesystem".to_string())
    );

    // Simulate tool usage - unregister after use
    manager.unregister_tool("read_file").await;
    assert_eq!(manager.list_tools().await.len(), 2);
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
    let manager = DynamicToolManager::new();
    let manager_clone = manager.clone();

    // Spawn concurrent registration tasks
    let handle1 = tokio::spawn(async move {
        for i in 0..10 {
            manager
                .register_tool(
                    "server1".to_string(),
                    format!("tool_{}", i),
                    format!("Tool {}", i),
                    json!({"type": "object"}),
                )
                .await;
        }
    });

    let handle2 = tokio::spawn(async move {
        for i in 10..20 {
            manager_clone
                .register_tool(
                    "server2".to_string(),
                    format!("tool_{}", i),
                    format!("Tool {}", i),
                    json!({"type": "object"}),
                )
                .await;
        }
    });

    // Wait for both to complete
    let _ = tokio::join!(handle1, handle2);

    // Verify all tools registered
    let tools = manager.list_tools().await;
    assert_eq!(tools.len(), 20);
}

#[tokio::test]
async fn test_tool_lifecycle() {
    let manager = DynamicToolManager::new();

    // Phase 1: Initial state - no tools
    assert_eq!(manager.list_tools().await.len(), 0);
    assert!(!manager.has_tool("read_file").await);

    // Phase 2: Register a tool
    let is_new = manager
        .register_tool(
            "filesystem".to_string(),
            "read_file".to_string(),
            "Read a file".to_string(),
            json!({"type": "object"}),
        )
        .await;
    assert!(is_new);
    assert_eq!(manager.list_tools().await.len(), 1);
    assert!(manager.has_tool("read_file").await);

    // Phase 3: Update same tool (not new)
    let is_new = manager
        .register_tool(
            "filesystem".to_string(),
            "read_file".to_string(),
            "Read a file (updated)".to_string(),
            json!({"type": "object"}),
        )
        .await;
    assert!(!is_new);
    assert_eq!(manager.list_tools().await.len(), 1);

    // Phase 4: Remove the tool
    let removed = manager.unregister_tool("read_file").await;
    assert!(removed);
    assert_eq!(manager.list_tools().await.len(), 0);
    assert!(!manager.has_tool("read_file").await);
}

#[tokio::test]
async fn test_multi_server_routing() {
    let manager = DynamicToolManager::new();

    // Register tools from multiple servers
    let servers = vec![
        ("filesystem", vec!["read_file", "write_file"]),
        ("git", vec!["commit", "push", "pull"]),
        ("search", vec!["web_search", "doc_search"]),
    ];

    for (server, tools) in servers {
        for tool in tools {
            manager
                .register_tool(
                    server.to_string(),
                    tool.to_string(),
                    format!("{} from {}", tool, server),
                    json!({"type": "object"}),
                )
                .await;
        }
    }

    // Verify total tool count
    assert_eq!(manager.list_tools().await.len(), 7);

    // Verify correct server routing
    assert_eq!(
        manager.get_server("read_file").await,
        Some("filesystem".to_string())
    );
    assert_eq!(
        manager.get_server("commit").await,
        Some("git".to_string())
    );
    assert_eq!(
        manager.get_server("web_search").await,
        Some("search".to_string())
    );

    // Clear filesystem tools
    manager.unregister_tool("read_file").await;
    manager.unregister_tool("write_file").await;
    assert_eq!(manager.list_tools().await.len(), 5);

    // Clear all tools
    manager.clear().await;
    assert_eq!(manager.list_tools().await.len(), 0);
}
