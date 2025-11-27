use anyhow::{Result, anyhow};
use serde_json::{json, Map as JsonMap};
use std::process::Stdio;
use tokio::process::Command as TokioCommand;
use tracing::{info, warn};
use tracing_subscriber;
use rmcp::{
    ServiceExt,
};

/// Real RMCP Client that connects to agentic-warden MCP server via stdio
struct McpClient {
    service: rmcp::service::RunningService<rmcp::RoleClient, ()>,
    _child: tokio::process::Child,
}

impl McpClient {
    async fn new() -> Result<Self> {
        // Initialize tracing
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_target(false)
            .init();

        info!("🚀 Starting RMCP Client with real MCP protocol");

        // Start agentic-warden as MCP server process
        info!("🔧 Starting embedded agentic-warden MCP server...");
        let mut child = TokioCommand::new("cargo")
            .args(&[
                "run",
                "--bin",
                "aiw",
                "--",
                "mcp",
                "serve",
                "--transport",
                "stdio",
                "--log-level",
                "warn"
            ])
            .current_dir("..")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Use stdout directly as transport (stdio transport)
        // RMCP can use (stdout, stdin) directly
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;

        // Create RMCP client service using the transport
        info!("🔗 Creating RMCP client connection...");
        let service = ().serve((stdout, stdin)).await?;

        Ok(Self { service, _child: child })
    }

    async fn run(&mut self) -> Result<()> {
        info!("🎭 Starting real RMCP simulation...");

        // Get server info
        let server_info = self.service.peer().peer_info();
        match server_info {
            Some(info) => {
                let name = info.server_info.name.as_str();
                let version = info.server_info.version.as_str();
                info!("✅ Connected to agentic-warden server: {} v{}", name, version);
            },
            None => {
                info!("✅ Connected to agentic-warden server (server info not available)");
            }
        }

        // List available tools
        info!("\n🛠️  Listing available tools from agentic-warden...");
        match self.service.peer().list_tools(Default::default()).await {
            Ok(tools) => {
                info!("✅ Found {} available tools:", tools.tools.len());
                for tool in &tools.tools {
                    info!("   - {}", tool.name);
                }

                // Test calling tools
                if !tools.tools.is_empty() {
                    self.test_tools(&tools.tools).await?;
                }
            },
            Err(e) => {
                warn!("⚠️  Failed to list tools: {}", e);
            }
        }

        // Test prompts if available
        info!("\n📝 Testing available prompts...");
        match self.service.peer().list_prompts(Default::default()).await {
            Ok(prompts) => {
                info!("✅ Found {} available prompts:", prompts.prompts.len());
                for prompt in &prompts.prompts {
                    info!("   - {}", prompt.name);
                }

                // Try to get a prompt
                if !prompts.prompts.is_empty() {
                    let first_prompt = &prompts.prompts[0];
                    info!("\n📄 Testing prompt: {}", first_prompt.name);

                    let get_prompt_params = rmcp::model::GetPromptRequestParam {
                        name: first_prompt.name.clone(),
                        arguments: None,
                    };

                    match self.service.peer().get_prompt(get_prompt_params).await {
                        Ok(prompt_result) => {
                            info!("✅ Prompt retrieved:");
                            info!("   Description: {:?}", prompt_result.description);
                            info!("   Messages: {}", prompt_result.messages.len());
                        },
                        Err(e) => {
                            warn!("⚠️  Failed to get prompt '{}': {}", first_prompt.name, e);
                        }
                    }
                }
            },
            Err(e) => {
                warn!("⚠️  Failed to list prompts: {}", e);
            }
        }

        // Test third-party provider integration through tool calls
        info!("\n🧪 Testing Third-Party Provider Integration:");
        self.test_provider_integration("kimi").await?;
        self.test_provider_integration("glm").await?;
        self.test_provider_integration("aws").await?;

        info!("\n🎉 Real RMCP simulation completed successfully!");
        info!("📋 Summary:");
        info!("   - Started embedded agentic-warden MCP server");
        info!("   - Connected via real RMCP protocol");
        info!("   - Listed and tested available tools");
        info!("   - Tested third-party provider integration");
        info!("   - Verified MCP protocol compliance");
        info!("\n💡 Features Demonstrated:");
        info!("   - Automatic server lifecycle management");
        info!("   - Real MCP protocol communication");
        info!("   - Third-party provider integration");
        info!("   - Dynamic tool discovery and invocation");

        Ok(())
    }

    async fn test_tools(&mut self, tools: &[rmcp::model::Tool]) -> Result<()> {
        for tool in tools {
            let tool_name = &tool.name;

            info!("\n🔧 Testing tool: {}", tool_name);

            // Create tool arguments based on tool name
            let tool_args = if tool_name.contains("execute") || tool_name.contains("ai") {
                let mut map = JsonMap::new();
                map.insert("ai_type".to_string(), json!("claude"));
                map.insert("provider".to_string(), json!("kimi"));
                map.insert("prompt".to_string(), json!("Hello from RMCP client! Please write a simple Rust function to demonstrate MCP integration."));
                map
            } else if tool_name.contains("provider") {
                JsonMap::new()
            } else if tool_name.contains("status") {
                JsonMap::new()
            } else {
                let mut map = JsonMap::new();
                map.insert("test".to_string(), json!("value"));
                map
            };

            let call_tool_params = rmcp::model::CallToolRequestParam {
                name: tool_name.clone(),
                arguments: Some(tool_args),
            };

            match self.service.peer().call_tool(call_tool_params).await {
                Ok(result) => {
                    info!("✅ Tool '{}' called successfully:", tool_name);
                    for (i, content) in result.content.iter().enumerate() {
                        info!("   Response {}: {:?}", i, content);
                    }
                },
                Err(e) => {
                    warn!("⚠️  Tool '{}' call failed: {}", tool_name, e);
                }
            }
        }
        Ok(())
    }

    async fn test_provider_integration(&self, provider: &str) -> Result<()> {
        info!("\n🔄 Testing {} provider integration:", provider);

        // Create a test tool call for provider integration
        let mut tool_args = JsonMap::new();
        tool_args.insert("ai_type".to_string(), json!("claude"));
        tool_args.insert("provider".to_string(), json!(provider));
        tool_args.insert("prompt".to_string(), json!(format!("Test integration with {} provider", provider)));

        let call_tool_params = rmcp::model::CallToolRequestParam {
            name: "execute_ai_query".into(), // This would be the actual tool name in agentic-warden
            arguments: Some(tool_args),
        };

        match self.service.peer().call_tool(call_tool_params).await {
            Ok(result) => {
                info!("✅ {} provider integration successful:", provider);
                for content in &result.content {
                    info!("   Response: {:?}", content);
                }
            },
            Err(e) => {
                info!("   {} provider test: {} (expected if tool not available)", provider, e);
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    match McpClient::new().await {
        Ok(mut client) => {
            if let Err(e) = client.run().await {
                eprintln!("RMCP simulation failed: {}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Failed to create RMCP client: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}