use super::pool::DiscoveredTool;
use anyhow::Result;
use std::collections::HashMap;

/// 能力描述生成器：使用确定性模板生成，保证工具名称完整不遗漏。
pub struct CapabilityGenerator;

impl CapabilityGenerator {
    pub fn new() -> Self {
        Self
    }

    /// 生成统一的能力描述（确定性模板，包含所有工具名称）
    pub fn generate_capability_description(
        &self,
        discovered_tools: &[DiscoveredTool],
    ) -> Result<String> {
        let stats = self.analyze_tools(discovered_tools);
        Ok(self.generate_with_template(&stats))
    }

    /// 分析工具，提取统计信息
    fn analyze_tools(&self, discovered_tools: &[DiscoveredTool]) -> ToolStats {
        let mut servers = std::collections::HashSet::new();
        let mut categories = HashMap::new();
        let mut tools_by_server: HashMap<String, Vec<String>> = HashMap::new();

        for tool in discovered_tools {
            servers.insert(tool.server.clone());

            tools_by_server
                .entry(tool.server.clone())
                .or_default()
                .push(tool.definition.name.to_string());

            let description_str = tool.definition.description.as_ref().map(|s| s.as_ref());
            let category = self.infer_category(&tool.definition.name, description_str);
            *categories.entry(category).or_insert(0) += 1;
        }

        ToolStats {
            server_count: servers.len(),
            server_names: servers.into_iter().collect(),
            tool_count: discovered_tools.len(),
            categories,
            tools_by_server,
        }
    }

    /// 从工具名称和描述中推断类别
    fn infer_category(&self, tool_name: &str, description: Option<&str>) -> String {
        let text = format!(
            "{} {}",
            tool_name.to_lowercase(),
            description.map(|s| s.to_lowercase()).unwrap_or_default()
        );

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

    /// 使用模板生成能力描述
    fn generate_with_template(&self, stats: &ToolStats) -> String {
        let categories_str = stats
            .categories
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");

        let tools_listing: Vec<String> = stats
            .server_names
            .iter()
            .filter_map(|server| {
                stats.tools_by_server.get(server).map(|tools| {
                    format!("[{}] {}", server, tools.join(", "))
                })
            })
            .collect();

        format!(
            "I can route your requests to {} downstream MCP server{} ({}) with {} total tool{} available. Available tools: {}. Supported categories: {}.",
            stats.server_count,
            if stats.server_count > 1 { "s" } else { "" },
            stats.server_names.join(", "),
            stats.tool_count,
            if stats.tool_count > 1 { "s" } else { "" },
            tools_listing.join("; "),
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
    tools_by_server: HashMap<String, Vec<String>>,
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

    #[test]
    fn test_generate_description() {
        let generator = CapabilityGenerator::new();
        let tools = vec![
            create_test_tool("filesystem", "read_file", "Read a file from disk"),
            create_test_tool("filesystem", "write_file", "Write a file to disk"),
            create_test_tool("memory", "store_data", "Store data in memory"),
        ];

        let description = generator
            .generate_capability_description(&tools)
            .unwrap();

        assert!(description.contains("2 downstream MCP servers"));
        assert!(description.contains("3 total tools"));
        assert!(description.contains("filesystem"));
        assert!(description.contains("memory"));
        assert!(description.contains("read_file"), "should contain tool name read_file");
        assert!(description.contains("write_file"), "should contain tool name write_file");
        assert!(description.contains("store_data"), "should contain tool name store_data");
    }

    #[test]
    fn test_infer_category() {
        let generator = CapabilityGenerator::new();

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
        let generator = CapabilityGenerator::new();
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
        assert_eq!(stats.tools_by_server["filesystem"].len(), 2);
        assert_eq!(stats.tools_by_server["git"].len(), 1);
    }
}
