//! Integration tests for DynamicToolRegistry behaviors (TTL, eviction, performance).

use aiw::mcp_routing::registry::{DynamicToolRegistry, RegistryConfig};
use rmcp::model::Tool;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn make_tool(name: &str) -> Tool {
    let mut schema = serde_json::Map::new();
    schema.insert("type".into(), serde_json::Value::String("object".into()));

    Tool {
        name: name.to_string().into(),
        title: None,
        description: Some(format!("Tool {name}").into()),
        input_schema: Arc::new(schema),
        output_schema: None,
        icons: None,
        annotations: None,
    }
}

#[tokio::test]
async fn test_ttl_expiration_via_background_cleanup() {
    let registry = Arc::new(DynamicToolRegistry::with_config(
        vec![],
        RegistryConfig {
            default_ttl_seconds: 1,
            max_dynamic_tools: 10,
            cleanup_interval_seconds: 1,
        },
    ));

    registry
        .register_proxied_tool(
            "filesystem".to_string(),
            "temp_tool".to_string(),
            make_tool("temp_tool"),
        )
        .await
        .unwrap();

    let cleanup_task = registry.start_cleanup_task();
    tokio::time::sleep(Duration::from_secs(3)).await;
    cleanup_task.abort();
    let _ = cleanup_task.await;

    assert!(!registry.has_tool("temp_tool").await);
}

#[tokio::test]
async fn test_registry_eviction_strategy() {
    let registry = DynamicToolRegistry::with_config(
        vec![],
        RegistryConfig {
            default_ttl_seconds: 60,
            max_dynamic_tools: 5,
            cleanup_interval_seconds: 60,
        },
    );

    for idx in 0..5 {
        registry
            .register_proxied_tool(
                "server".to_string(),
                format!("tool_{idx}"),
                make_tool(&format!("tool_{idx}")),
            )
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    registry
        .register_proxied_tool(
            "server".to_string(),
            "tool_new".to_string(),
            make_tool("tool_new"),
        )
        .await
        .unwrap();

    assert!(
        !registry.has_tool("tool_0").await,
        "oldest tool should be evicted"
    );
    assert!(registry.has_tool("tool_new").await);
    assert_eq!(registry.get_all_tool_definitions().await.len(), 5);
}

#[tokio::test]
async fn test_registry_read_performance() {
    let registry = DynamicToolRegistry::new(vec![]);
    for idx in 0..50 {
        registry
            .register_proxied_tool(
                "server".to_string(),
                format!("tool_{idx}"),
                make_tool(&format!("tool_{idx}")),
            )
            .await
            .unwrap();
    }

    let iterations = 200;
    let start = Instant::now();
    for _ in 0..iterations {
        let snapshot = registry.get_all_tool_definitions().await;
        assert_eq!(snapshot.len(), 50);
    }
    let avg_ms = (start.elapsed().as_secs_f64() * 1000.0) / iterations as f64;
    assert!(
        avg_ms < 50.0,
        "Registry read average {avg_ms}ms exceeds 50ms target"
    );
}
