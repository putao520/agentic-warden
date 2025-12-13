use super::{
    source::RegistrySource,
    types::{EnvVarSpec, McpServerDetail, McpServerInfo, ServerInstallType},
};
use crate::commands::mcp::McpServerConfig;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};

const DEFAULT_BASE_URL: &str = "https://registry.modelcontextprotocol.io";

pub struct OfficialRegistrySource {
    client: Client,
    base_url: String,
}

impl OfficialRegistrySource {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL, None)
    }

    pub fn with_base_url(base_url: impl Into<String>, client: Option<Client>) -> Self {
        let http_client = client.unwrap_or_else(|| {
            Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build reqwest client")
        });

        Self {
            client: http_client,
            base_url: base_url.into(),
        }
    }

    fn build_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    async fn fetch_servers(&self, query: &str, limit: usize) -> Result<Vec<OfficialServerEnvelope>> {
        let url = format!(
            "{}?search={}&limit={}",
            self.build_url("/v0.1/servers"),
            urlencoding::encode(query),
            limit
        );

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to request official registry")?
            .error_for_status()
            .context("Official registry returned an error status")?;

        let parsed: OfficialSearchResponse = resp
            .json()
            .await
            .context("Failed to parse official registry response")?;

        Ok(parsed.servers)
    }

    fn entry_to_info(&self, entry: &OfficialServerEnvelope) -> Option<McpServerInfo> {
        let package = self.pick_package(&entry.server.packages)?;
        let install = self.package_to_install_type(&package)?;

        let qualified_name = if package.identifier.is_empty() {
            entry.server.name.clone()
        } else {
            package.identifier.clone()
        };
        let display_name = entry
            .server
            .title
            .clone()
            .or_else(|| qualified_name.rsplit('/').next().map(|part| part.to_string()))
            .unwrap_or_else(|| qualified_name.clone());

        Some(McpServerInfo {
            qualified_name,
            display_name,
            description: entry.server.description.clone(),
            source: self.source_id().to_string(),
            install,
            author: None,
            downloads: None,
        })
    }

    fn entry_to_detail(&self, entry: OfficialServerEnvelope) -> Option<McpServerDetail> {
        let info = self.entry_to_info(&entry)?;
        let package = self.pick_package(&entry.server.packages)?;
        let required_env = package
            .package_arguments
            .unwrap_or_default()
            .into_iter()
            .map(|arg| EnvVarSpec {
                name: arg.name.to_ascii_uppercase().replace('-', "_"),
                description: arg.description,
                required: arg.is_required,
                default: arg.default,
            })
            .collect();

        let repository = entry
            .server
            .repository
            .and_then(|repo| repo.url.or(repo.base_url));

        Some(McpServerDetail {
            info,
            repository,
            required_env,
        })
    }

    fn matches_entry(&self, entry: &OfficialServerEnvelope, target: &str) -> bool {
        let target_lower = target.to_lowercase();
        if entry.server.name.to_lowercase() == target_lower {
            return true;
        }
        if entry
            .server
            .title
            .as_ref()
            .map(|t| t.to_lowercase() == target_lower)
            .unwrap_or(false)
        {
            return true;
        }
        entry
            .server
            .packages
            .iter()
            .any(|pkg| pkg.identifier.to_lowercase() == target_lower)
    }

    fn pick_package(&self, packages: &[OfficialPackage]) -> Option<OfficialPackage> {
        let mut best: Option<OfficialPackage> = None;
        for pkg in packages {
            let candidate = match pkg.registry_type.as_str() {
                "npm" | "node" | "pypi" | "oci" | "mcpb" => Some(pkg.clone()),
                _ => None,
            };

            if let Some(c) = candidate {
                if let Some(existing) = &best {
                    if self.package_priority(&c) < self.package_priority(existing) {
                        best = Some(c);
                    }
                } else {
                    best = Some(c);
                }
            }
        }
        best
    }

    fn package_priority(&self, pkg: &OfficialPackage) -> u8 {
        match pkg.registry_type.as_str() {
            "npm" => 1,
            "pypi" => 2,
            "node" => 3,
            "oci" => 4,
            "mcpb" => 5,
            _ => 10,
        }
    }

    fn package_to_install_type(&self, pkg: &OfficialPackage) -> Option<ServerInstallType> {
        match pkg.registry_type.as_str() {
            "npm" | "node" => {
                if pkg
                    .runtime_hint
                    .as_ref()
                    .is_some_and(|hint| hint.eq_ignore_ascii_case("uvx"))
                {
                    Some(ServerInstallType::Uvx {
                        package: pkg.identifier.clone(),
                    })
                } else {
                    Some(ServerInstallType::Npm {
                        package: pkg.identifier.clone(),
                    })
                }
            }
            "pypi" => Some(ServerInstallType::Uvx {
                package: pkg.identifier.clone(),
            }),
            "oci" => Some(ServerInstallType::Docker {
                image: pkg.identifier.clone(),
            }),
            "mcpb" => pkg
                .transport
                .as_ref()
                .and_then(|t| t.url.clone())
                .map(|url| ServerInstallType::Remote { url }),
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl RegistrySource for OfficialRegistrySource {
    fn source_name(&self) -> &'static str {
        "Official MCP Registry"
    }

    fn source_id(&self) -> &'static str {
        "registry"
    }

    fn priority(&self) -> u8 {
        1
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<McpServerInfo>> {
        let entries = self.fetch_servers(query, limit).await?;
        let infos = entries
            .iter()
            .filter_map(|entry| self.entry_to_info(entry))
            .collect::<Vec<_>>();
        Ok(infos)
    }

    async fn get_server(&self, name: &str) -> Result<Option<McpServerDetail>> {
        let target = name.trim_start_matches("registry:");
        let entries = self.fetch_servers(target, 20).await?;
        for entry in entries {
            if self.matches_entry(&entry, target) {
                return Ok(self.entry_to_detail(entry));
            }
        }
        Ok(None)
    }

    async fn get_install_config(&self, name: &str) -> Result<McpServerConfig> {
        let detail = self
            .get_server(name)
            .await?
            .ok_or_else(|| anyhow!("Server '{}' not found in official registry", name))?;

        let (command, args) = detail.info.install.command_and_args();
        let mut env = HashMap::new();
        for spec in &detail.required_env {
            if let Some(default_value) = &spec.default {
                env.insert(spec.name.clone(), default_value.clone());
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
struct OfficialSearchResponse {
    #[serde(default)]
    servers: Vec<OfficialServerEnvelope>,
}

#[derive(Debug, Deserialize, Clone)]
struct OfficialServerEnvelope {
    server: OfficialServer,
}

#[derive(Debug, Deserialize, Clone)]
struct OfficialServer {
    name: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    packages: Vec<OfficialPackage>,
    #[serde(default)]
    repository: Option<OfficialRepository>,
}

#[derive(Debug, Deserialize, Clone)]
struct OfficialRepository {
    #[serde(default)]
    url: Option<String>,
    #[serde(rename = "baseUrl")]
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct OfficialPackage {
    #[serde(rename = "registryType")]
    registry_type: String,
    identifier: String,
    #[serde(default)]
    runtime_hint: Option<String>,
    #[serde(default)]
    transport: Option<OfficialTransport>,
    #[serde(rename = "packageArguments")]
    #[serde(default)]
    package_arguments: Option<Vec<OfficialPackageArgument>>,
}

#[derive(Debug, Deserialize, Clone)]
struct OfficialTransport {
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct OfficialPackageArgument {
    name: String,
    #[serde(rename = "isRequired")]
    #[serde(default)]
    is_required: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    default: Option<String>,
}
