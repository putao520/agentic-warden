use aiw::commands::mcp::registry::{
    aggregator::RegistryAggregator,
    official::OfficialRegistrySource,
    smithery::SmitherySource,
    source::RegistrySource,
    types::{McpServerInfo, ServerInstallType},
};
use aiw::commands::mcp::registry::types::McpServerDetail;
use aiw::commands::mcp::McpServerConfig;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use mockito::Matcher;

#[tokio::test]
async fn official_source_maps_identifier_and_env() {
    let mut server = mockito::Server::new_async().await;
    let body = r#"{
        "servers": [{
            "server": {
                "name": "io.test/sample",
                "description": "Sample server",
                "packages": [{
                    "registryType": "npm",
                    "identifier": "@test/sample",
                    "packageArguments": [{
                        "name": "API_KEY",
                        "isRequired": true,
                        "description": "Key description"
                    }]
                }]
            }
        }]
    }"#;

    let _m = server
        .mock("GET", "/v0.1/servers")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_body(body)
        .create();

    let source = OfficialRegistrySource::with_base_url(server.url(), None);
    let detail = source
        .get_server("@test/sample")
        .await
        .expect("request should succeed")
        .expect("server should be found");

    assert_eq!(detail.info.qualified_name, "@test/sample");
    assert_eq!(detail.info.install.label(), "npm");
    assert_eq!(detail.required_env.len(), 1);
    assert_eq!(detail.required_env[0].name, "API_KEY");
}

#[tokio::test]
async fn smithery_source_parses_basic_fields() {
    let mut server = mockito::Server::new_async().await;
    let body = r#"{
        "servers": [{
            "id": "exa",
            "slug": "exa",
            "description": "Search server",
            "packageName": "@smithery/exa",
            "serverType": "npm",
            "env": [{
                "name": "EXA_API_KEY",
                "required": true
            }]
        }],
        "totalCount": 1
    }"#;

    let _m = server
        .mock("GET", "/servers")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_body(body)
        .create();

    let source = SmitherySource::with_base_url(server.url(), None, None);
    let detail = source
        .get_server("smithery:exa")
        .await
        .expect("request should succeed")
        .expect("server should be found");

    assert_eq!(detail.info.qualified_name, "smithery:exa");
    assert_eq!(detail.info.install.label(), "npm");
    assert_eq!(detail.required_env.len(), 1);
    assert_eq!(detail.required_env[0].name, "EXA_API_KEY");
}

#[tokio::test]
async fn aggregator_prefers_higher_priority_sources() -> Result<()> {
    let high_priority = StubSource {
        id: "registry",
        priority: 1,
        result: vec![McpServerInfo {
            qualified_name: "shared".to_string(),
            display_name: "shared".to_string(),
            description: None,
            source: "registry".to_string(),
            install: ServerInstallType::Npm {
                package: "@test/a".to_string(),
            },
            author: None,
            downloads: Some(50),
        }],
    };

    let low_priority = StubSource {
        id: "smithery",
        priority: 2,
        result: vec![
            McpServerInfo {
                qualified_name: "shared".to_string(),
                display_name: "shared-low".to_string(),
                description: None,
                source: "smithery".to_string(),
                install: ServerInstallType::Npm {
                    package: "@test/b".to_string(),
                },
                author: None,
                downloads: Some(100),
            },
            McpServerInfo {
                qualified_name: "unique".to_string(),
                display_name: "unique".to_string(),
                description: None,
                source: "smithery".to_string(),
                install: ServerInstallType::Npm {
                    package: "@test/c".to_string(),
                },
                author: None,
                downloads: Some(10),
            },
        ],
    };

    let aggregator =
        RegistryAggregator::with_sources(vec![Box::new(high_priority), Box::new(low_priority)]);
    let results = aggregator.search("query", None, 10).await?;

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].qualified_name, "shared");
    assert_eq!(results[0].source, "registry");
    assert_eq!(results[1].qualified_name, "unique");
    Ok(())
}

struct StubSource {
    id: &'static str,
    priority: u8,
    result: Vec<McpServerInfo>,
}

#[async_trait]
impl RegistrySource for StubSource {
    fn source_name(&self) -> &'static str {
        self.id
    }

    fn source_id(&self) -> &'static str {
        self.id
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<McpServerInfo>> {
        Ok(self.result.clone())
    }

    async fn get_server(&self, _name: &str) -> Result<Option<McpServerDetail>> {
        Ok(None)
    }

    async fn get_install_config(&self, _name: &str) -> Result<McpServerConfig> {
        Err(anyhow!("not implemented"))
    }
}
