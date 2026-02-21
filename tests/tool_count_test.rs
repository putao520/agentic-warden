//! Test to check how many tools are registered at startup

#[cfg(test)]
mod tests {
    use aiw::mcp::AgenticWardenMcpServer;
    use anyhow::Result;

    #[tokio::test]
    #[ignore = "requires MCP servers configured in mcp.json"]
    async fn test_tool_count_at_startup() -> Result<()> {
        println!("🔍 Checking tool count at startup...");

        // Create the MCP server to see all tools (base + dynamic)
        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        let all_tools = server.get_all_tool_definitions().await;

        println!("📊 Total tools count: {}", all_tools.len());

        // Expected base tools (current API)
        let expected_base_tools = vec![
            "intelligent_route",
            "start_task",
            "stop_task",
            "list_tasks",
            "get_task_logs",
            "get_task_status",
            "list_roles",
            "list_providers",
        ];

        // Find base tools (static ones we know about)
        let mut base_tools_found = 0;
        println!("\n📋 Expected base tools verification:");
        for expected_tool in &expected_base_tools {
            if all_tools.iter().any(|t| &*t.name == *expected_tool) {
                println!("  ✅ Found: {}", expected_tool);
                base_tools_found += 1;
            } else {
                println!("  ❌ Missing: {}", expected_tool);
            }
        }

        // Print all tool names
        println!("\n📋 All registered tools:");
        for (i, tool) in all_tools.iter().enumerate() {
            println!("  {}. {}", i + 1, tool.name);
        }

        println!("\n📈 Tool breakdown:");
        println!("  - Expected base tools: {}", expected_base_tools.len());
        println!("  - Found base tools: {}", base_tools_found);
        println!("  - Total: {}", all_tools.len());

        // Verify we have all expected base tools
        assert_eq!(
            base_tools_found,
            expected_base_tools.len(),
            "Should have all expected base tools"
        );

        // Verify total tool count matches base tools
        assert_eq!(
            all_tools.len(),
            expected_base_tools.len(),
            "Total tools should equal base tools at startup"
        );

        println!("\n✅ Tool count verification: PASSED");
        println!("🎉 All {} base tools found", expected_base_tools.len());

        Ok(())
    }
}
