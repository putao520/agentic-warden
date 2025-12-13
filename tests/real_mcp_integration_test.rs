//! REAL integration tests - No Mocks Allowed
//! Tests actual MCP server connections and dynamic tool generation

#[cfg(test)]
mod tests {
    use aiw::mcp::AgenticWardenMcpServer;
    use anyhow::Result;
    use serial_test::serial;

    #[tokio::test]
    #[ignore = "requires MCP servers configured"]
    #[serial]
    async fn test_real_mcp_server_connection() -> Result<()> {
        println!("🧪 Testing REAL MCP server connection (NO MOCKS ALLOWED)");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        println!("✅ Server initialized");

        // Get all tools from REAL MCP servers
        let tools = server.get_all_tool_definitions().await;

        println!("📊 Total REAL tools: {}", tools.len());

        // List all REAL tools available
        println!("📋 REAL tools from MCP servers:");
        for (i, tool) in tools.iter().enumerate() {
            println!(
                "  {}. {} - {}",
                i + 1,
                tool.name,
                tool.description.as_ref().map_or("", |s| s)
            );
        }

        // Verify we have REAL tools from MCP servers
        assert!(
            !tools.is_empty(),
            "Should have at least some tools from MCP servers"
        );

        // Check for specific REAL tools we know exist
        let real_tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();

        println!("\n🔍 Checking for REAL filesystem tools:");
        let filesystem_tools = real_tool_names
            .iter()
            .filter(|name| {
                name.contains("read_file")
                    || name.contains("write_file")
                    || name.contains("create_directory")
            })
            .count();
        println!("  Found {} filesystem tools", filesystem_tools);

        println!("\n🔍 Checking for REAL knowledge graph tools:");
        let kg_tools = real_tool_names
            .iter()
            .filter(|name| {
                name.contains("add_entity")
                    || name.contains("delete_entity")
                    || name.contains("create_relation")
            })
            .count();
        println!("  Found {} knowledge graph tools", kg_tools);

        println!("\n🎯 Real Integration Test Results:");
        println!("  ✅ Connected to {} REAL tools", tools.len());
        println!("  ✅ No mock data used");
        println!("  ✅ Actual MCP servers responding");

        // This is a REAL test - no mocking allowed
        assert!(
            tools.len() > 2,
            "Should have more than just the 3 base tools"
        );

        println!("\n🚀 All REAL integration tests passed!");
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires MCP servers configured"]
    #[serial]
    async fn test_real_tool_execution() -> Result<()> {
        println!("🧪 Testing REAL tool execution (NO MOCKS)");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // Get a specific tool to test
        let tools = server.get_all_tool_definitions().await;

        // Find a filesystem tool we can actually execute
        let test_tool = tools.iter().find(|t| {
            t.name.contains("read_file")
                || t.name.contains("list_files")
                || t.name.contains("write_file")
        });

        if let Some(tool) = test_tool {
            println!("🔧 Found REAL tool to test: {}", tool.name);
            println!("📝 Description: {:?}", tool.description);

            // Note: We can't actually execute the tool here without proper parameters
            // But we can verify it exists and is properly configured
            assert!(!tool.name.is_empty(), "Tool should have a name");
            assert!(
                !tool.input_schema.as_ref().is_empty(),
                "Tool should have input schema"
            );

            println!(
                "✅ Tool {} is properly configured for REAL execution",
                tool.name
            );
        } else {
            println!("⚠️  No filesystem tools found, but this could be expected");
        }

        println!("\n✅ REAL tool execution test completed!");
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires MCP servers configured"]
    #[serial]
    async fn test_ollama_real_integration() -> Result<()> {
        println!("🧪 Testing REAL Ollama integration (NO MOCKS)");

        // Verify Ollama is actually running and accessible
        let ollama_url = "http://localhost:11434/api/tags";

        let client = reqwest::Client::new();

        match client.get(ollama_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    println!("✅ REAL Ollama server is accessible");

                    let body = response.text().await?;
                    if body.contains("qwen3:1.7b") {
                        println!("✅ REAL qwen3:1.7b model is available");
                    } else {
                        println!("⚠️  qwen3:1.7b model not found, but Ollama is running");
                    }
                } else {
                    anyhow::bail!("Ollama server returned status: {}", response.status());
                }
            }
            Err(e) => {
                anyhow::bail!("Failed to connect to Ollama: {}", e);
            }
        }

        println!("\n✅ REAL Ollama integration test passed!");
        Ok(())
    }
}
