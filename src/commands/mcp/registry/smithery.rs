use super::{
    source::RegistrySource,
    types::{EnvVarSpec, McpServerDetail, McpServerInfo, ServerInstallType},
};
use crate::commands::mcp::McpServerConfig;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};

const DEFAULT_BASE_URL: &str = "https://registry.smithery.ai";

pub struct SmitherySource {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl SmitherySource {
    pub fn new() -> Self {
        let api_key = std::env::var("SMITHERY_API_KEY").ok();
        Self::with_base_url(DEFAULT_BASE_URL, api_key, None)
    }

    pub fn with_base_url(
        base_url: impl Into<String>,
        api_key: Option<String>,
        client: Option<Client>,
    ) -> Self {
        let http_client = client.unwrap_or_else(|| {
            Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build reqwest client")
        });

        Self {
            client: http_client,
            base_url: base_url.into(),
            api_key,
        }
    }

    fn build_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    async fn fetch_servers(&self, query: &str, limit: usize) -> Result<Vec<SmitheryServer>> {
        let url = format!(
            "{}?q={}&page=1&pageSize={}",
            self.build_url("/servers"),
            urlencoding::encode(query),
            limit
        );

        let mut req = self.client.get(url);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .context("Failed to request Smithery registry")?
            .error_for_status()
            .context("Smithery registry returned an error status")?;

        let parsed: SmitherySearchResponse = resp
            .json()
            .await
            .context("Failed to parse Smithery registry response")?;

        Ok(parsed.servers)
    }

    fn server_to_info(&self, server: &SmitheryServer) -> Option<McpServerInfo> {
        let install = self.detect_install_type(server)?;
        let qualified_name = format!("smithery:{}", server.identifier());
        let display_name = server
            .display_name
            .clone()
            .unwrap_or_else(|| server.identifier());

        Some(McpServerInfo {
            qualified_name,
            display_name,
            description: server.description.clone(),
            source: self.source_id().to_string(),
            install,
            author: server.author.clone(),
            downloads: server.downloads,
        })
    }

    fn server_to_detail(&self, server: SmitheryServer) -> Option<McpServerDetail> {
        let info = self.server_to_info(&server)?;
        let required_env = server
            .env
            .unwrap_or_default()
            .into_iter()
            .map(|var| EnvVarSpec {
                name: var.name,
                description: var.description,
                required: var.required.unwrap_or(false),
                default: var.default,
            })
            .collect();

        Some(McpServerDetail {
            info,
            repository: server.repository,
            required_env,
        })
    }

    fn detect_install_type(&self, server: &SmitheryServer) -> Option<ServerInstallType> {
        let type_hint = server
            .server_type
            .as_ref()
            .or(server.type_field.as_ref())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "npm".to_string());

        match type_hint.as_str() {
            "npm" => server.package_name.as_ref().map(|pkg| ServerInstallType::Npm {
                package: pkg.clone(),
            }),
            "uvx" | "pypi" => server.package_name.as_ref().map(|pkg| ServerInstallType::Uvx {
                package: pkg.clone(),
            }),
            "docker" | "oci" => server.package_name.as_ref().map(|pkg| ServerInstallType::Docker {
                image: pkg.clone(),
            }),
            "remote" => server
                .url
                .as_ref()
                .map(|url| ServerInstallType::Remote { url: url.clone() }),
            _ => server.package_name.as_ref().map(|pkg| ServerInstallType::Npm {
                package: pkg.clone(),
            }),
        }
    }
}

#[async_trait::async_trait]
impl RegistrySource for SmitherySource {
    fn source_name(&self) -> &'static str {
        "Smithery Registry"
    }

    fn source_id(&self) -> &'static str {
        "smithery"
    }

    fn priority(&self) -> u8 {
        2
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<McpServerInfo>> {
        let servers = self.fetch_servers(query, limit).await?;
        let infos = servers
            .iter()
            .filter_map(|server| self.server_to_info(server))
            .collect();
        Ok(infos)
    }

    async fn get_server(&self, name: &str) -> Result<Option<McpServerDetail>> {
        let query_name = name.trim_start_matches("smithery:");
        let servers = self.fetch_servers(query_name, 20).await?;
        for server in servers {
            if server.identifier().eq_ignore_ascii_case(query_name) {
                return Ok(self.server_to_detail(server));
            }
        }
        Ok(None)
    }

    async fn get_install_config(&self, name: &str) -> Result<McpServerConfig> {
        let detail = self
            .get_server(name)
            .await?
            .ok_or_else(|| anyhow!("Server '{}' not found in smithery registry", name))?;

        let (command, args) = detail.info.install.command_and_args();
        let mut env = HashMap::new();
        for spec in &detail.required_env {
            if let Some(default) = &spec.default {
                env.insert(spec.name.clone(), default.clone());
            }
        }

        Ok(McpServerConfig {
            command,
            args,
            env,
            description: detail.info.description.clone(),
            category: None,
            enabled: Some(true),
            source: Some(self.source_id().to_string()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct SmitherySearchResponse {
    #[serde(default)]
    servers: Vec<SmitheryServer>,
    #[serde(rename = "totalCount")]
    #[serde(default)]
    _total_count: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
struct SmitheryServer {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(rename = "packageName")]
    #[serde(default)]
    package_name: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(rename = "serverType")]
    #[serde(default)]
    server_type: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    type_field: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    downloads: Option<u64>,
    #[serde(default)]
    env: Option<Vec<SmitheryEnvVar>>,
}

impl SmitheryServer {
    fn identifier(&self) -> String {
        self.slug
            .clone()
            .or_else(|| self.name.clone())
            .or_else(|| self.id.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[derive(Debug, Deserialize, Clone)]
struct SmitheryEnvVar {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    default: Option<String>,
}
