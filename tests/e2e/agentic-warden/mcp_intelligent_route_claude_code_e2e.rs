//! MCP Intelligent Route - Claude Code Integration E2E Tests
//! Tests REQ-012: 智能MCP路由系统 (Claude Code集成)

mod common;

use anyhow::Result;
use common::start_mcp_server;
use serial_test::serial;
use serde_json::json;
use std::time::{Duration, Instant};

#[tokio::test]
#[serial]
async fn test_list_tools_dynamic_refresh() -> Result<()> {
    let mcp_server = start_mcp_server().await;

    let initial_tools = mcp_server.list_tools().await?;
    assert_eq!(initial_tools.len(), 2);

    let route_result = mcp_server
        .call_tool(
            "intelligent_route",
            json!({
                "user_request": "Check git status",
                "execution_mode": "dynamic"
            }),
        )
        .await?;

    assert!(route_result["dynamically_registered"]
        .as_bool()
        .unwrap_or(false));

    let updated_tools = mcp_server.list_tools().await?;
    assert!(updated_tools.len() > initial_tools.len());
    assert!(updated_tools.iter().any(|t| t.name.contains("git")));

    let refresh_start = Instant::now();
    let _ = mcp_server.list_tools().await?;
    assert!(refresh_start.elapsed() < Duration::from_secs(1));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_orchestrated_tool_execution_via_claude_code() -> Result<()> {
    let mcp_server = start_mcp_server().await;

    let route_result = mcp_server
        .call_tool(
            "intelligent_route",
            json!({
                "user_request": "Create a git report with status and commit summary",
                "execution_mode": "dynamic"
            }),
        )
        .await?;

    let tool_name = route_result["selected_tool"]["tool_name"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(!tool_name.is_empty());

    let tools = mcp_server.list_tools().await?;
    let orchestrated_tool = tools.iter().find(|t| t.name == tool_name);
    assert!(orchestrated_tool.is_some());

    let execution_result = mcp_server
        .call_tool(
            &tool_name,
            json!({
                "repo_path": "/tmp/test-repo"
            }),
        )
        .await?;

    assert!(execution_result["ok"].as_bool().unwrap_or(false));

    Ok(())
}
