use super::pool::DiscoveredTool;
use anyhow::{anyhow, Result};
use ollama_rs::{
    generation::completion::request::GenerationRequest, generation::completion::GenerationResponse,
    Ollama,
};
use std::collections::HashMap;

/// 能力描述生成器，支持双模式：
/// 1. LLM模式（优先）：使用 Ollama 生成智能描述
/// 2. 降级模式（保底）：使用字符串模板生成
pub struct CapabilityGenerator {
    llm_client: Option<Ollama>,
    model: Option<String>,
}

impl CapabilityGenerator {
    /// 创建 LLM 模式的生成器
    pub fn with_llm(endpoint: &str, model: &str) -> Result<Self> {
        let client = Ollama::new(endpoint.to_string(), 11434);
        Ok(Self {
            llm_client: Some(client),
            model: Some(model.to_string()),
        })
    }

    /// 创建降级模式的生成器（无 LLM 依赖）
    pub fn fallback() -> Self {
        Self {
            llm_client: None,
            model: None,
        }
    }

    /// 生成统一的能力描述
    ///
    /// # 参数
    /// - `discovered_tools`: 从下游 MCP 服务器发现的所有工具
    ///
    /// # 返回
    /// 统一的能力描述字符串，例如：
    /// "I can route your requests to 2 downstream MCP servers (filesystem, memory)
    /// with 23 total tools available. Supported categories: file_operations, data_storage."
    pub async fn generate_capability_description(
        &self,
        discovered_tools: &[DiscoveredTool],
    ) -> Result<String> {
        // 分析工具，提取统计信息
        let stats = self.analyze_tools(discovered_tools);

        // 尝试 LLM 模式
        if let Some(ref client) = self.llm_client {
            if let Some(ref model) = self.model {
                match self.generate_with_llm(client, model, &stats).await {
                    Ok(description) => {
                        eprintln!("✅ LLM模式生成能力描述成功");
                        return Ok(description);
                    }
                    Err(e) => {
                        eprintln!("⚠️  LLM模式失败: {}, 降级到模板模式", e);
                    }
                }
            }
        }

        // 降级到模板模式
        eprintln!("📝 使用模板模式生成能力描述");
        Ok(self.generate_with_template(&stats))
    }

    /// 分析工具，提取统计信息
    fn analyze_tools(&self, discovered_tools: &[DiscoveredTool]) -> ToolStats {
        let mut servers = std::collections::HashSet::new();
        let mut categories = HashMap::new();

        for tool in discovered_tools {
            servers.insert(tool.server.clone());

            // 从工具名称或描述中推断类别
            let description_str = tool.definition.description.as_ref().map(|s| s.as_ref());
            let category = self.infer_category(&tool.definition.name, description_str);
            *categories.entry(category).or_insert(0) += 1;
        }

        ToolStats {
            server_count: servers.len(),
            server_names: servers.into_iter().collect(),
            tool_count: discovered_tools.len(),
            categories,
        }
    }

    /// 从工具名称和描述中推断类别
    fn infer_category(&self, tool_name: &str, description: Option<&str>) -> String {
        let text = format!(
            "{} {}",
            tool_name.to_lowercase(),
            description.map(|s| s.to_lowercase()).unwrap_or_default()
        );

        // 简单的关键词匹配
        if text.contains("file")
            || text.contains("read")
            || text.contains("write")
            || text.contains("directory")
        {
            "file_operations".to_string()
        } else if text.contains("git") || text.contains("commit") || text.contains("branch") {
            "version_control".to_string()
        } else if text.contains("data") || text.contains("store") || text.contains("memory") {
            "data_storage".to_string()
        } else if text.contains("search") || text.contains("query") || text.contains("find") {
            "search".to_string()
        } else if text.contains("web") || text.contains("http") || text.contains("fetch") {
            "web_access".to_string()
        } else {
            "general".to_string()
        }
    }

    /// 使用 LLM 生成能力描述
    async fn generate_with_llm(
        &self,
        client: &Ollama,
        model: &str,
        stats: &ToolStats,
    ) -> Result<String> {
        let categories_str = stats
            .categories
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");

        let prompt = format!(
            r#"You are an AI assistant that summarizes MCP (Model Context Protocol) capabilities.

Generate a concise, single-paragraph description of the following capabilities:
- Number of downstream MCP servers: {}
- Server names: {}
- Total number of tools: {}
- Tool categories: {}

The description should:
1. Be 1-2 sentences long
2. Mention the number of servers and tools
3. List the main categories
4. Be written in a friendly, informative tone
5. Start with "I can route your requests to..."

Example: "I can route your requests to 2 downstream MCP servers (filesystem, memory) with 23 total tools available. Supported categories: file_operations, data_storage, search."

Generate the description now:"#,
            stats.server_count,
            stats.server_names.join(", "),
            stats.tool_count,
            categories_str
        );

        let request = GenerationRequest::new(model.to_string(), prompt);
        let response: GenerationResponse = client
            .generate(request)
            .await
            .map_err(|e| anyhow!("Ollama API error: {}", e))?;

        let description = response.response.trim().to_string();

        if description.is_empty() {
            return Err(anyhow!("LLM returned empty description"));
        }

        Ok(description)
    }

    /// 使用模板生成能力描述
    fn generate_with_template(&self, stats: &ToolStats) -> String {
        let categories_str = stats
            .categories
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "I can route your requests to {} downstream MCP server{} ({}) with {} total tool{} available. Supported categories: {}.",
            stats.server_count,
            if stats.server_count > 1 { "s" } else { "" },
            stats.server_names.join(", "),
            stats.tool_count,
            if stats.tool_count > 1 { "s" } else { "" },
            categories_str
        )
    }
}

/// 工具统计信息
#[derive(Debug)]
struct ToolStats {
    server_count: usize,
    server_names: Vec<String>,
    tool_count: usize,
    categories: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Tool;
    use serde_json::Map;
    use std::borrow::Cow;
    use std::sync::Arc;

    fn create_test_tool(server: &str, name: &str, description: &str) -> DiscoveredTool {
        DiscoveredTool {
            server: server.to_string(),
            definition: Tool {
                name: Cow::Owned(name.to_string()),
                title: None,
                description: Some(Cow::Owned(description.to_string())),
                input_schema: Arc::new(Map::new()),
                output_schema: None,
                icons: None,
                annotations: None,
                execution: None,
                meta: None,
            },
        }
    }

    #[tokio::test]
    async fn test_fallback_mode() {
        let generator = CapabilityGenerator::fallback();
        let tools = vec![
            create_test_tool("filesystem", "read_file", "Read a file from disk"),
            create_test_tool("filesystem", "write_file", "Write a file to disk"),
            create_test_tool("memory", "store_data", "Store data in memory"),
        ];

        let description = generator
            .generate_capability_description(&tools)
            .await
            .unwrap();

        assert!(description.contains("2 downstream MCP servers"));
        assert!(description.contains("3 total tools"));
        assert!(description.contains("filesystem"));
        assert!(description.contains("memory"));
    }

    #[test]
    fn test_infer_category() {
        let generator = CapabilityGenerator::fallback();

        assert_eq!(
            generator.infer_category("read_file", Some("Read a file")),
            "file_operations"
        );
        assert_eq!(
            generator.infer_category("git_commit", Some("Commit changes")),
            "version_control"
        );
        assert_eq!(
            generator.infer_category("store_data", Some("Store data")),
            "data_storage"
        );
        assert_eq!(
            generator.infer_category("search_query", Some("Search for items")),
            "search"
        );
    }

    #[test]
    fn test_analyze_tools() {
        let generator = CapabilityGenerator::fallback();
        let tools = vec![
            create_test_tool("filesystem", "read_file", "Read a file"),
            create_test_tool("filesystem", "write_file", "Write a file"),
            create_test_tool("git", "commit", "Commit changes"),
        ];

        let stats = generator.analyze_tools(&tools);

        assert_eq!(stats.server_count, 2);
        assert_eq!(stats.tool_count, 3);
        assert!(stats.server_names.contains(&"filesystem".to_string()));
        assert!(stats.server_names.contains(&"git".to_string()));
    }
}
