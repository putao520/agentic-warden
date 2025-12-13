use std::fmt;

/// Installation type supported by registry entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerInstallType {
    Npm { package: String },
    Uvx { package: String },
    Docker { image: String },
    Remote { url: String },
}

impl ServerInstallType {
    /// Returns a short label for table rendering.
    pub fn label(&self) -> &'static str {
        match self {
            ServerInstallType::Npm { .. } => "npm",
            ServerInstallType::Uvx { .. } => "uvx",
            ServerInstallType::Docker { .. } => "docker",
            ServerInstallType::Remote { .. } => "remote",
        }
    }

    /// Maps install type to the command/args used inside mcp.json.
    pub fn command_and_args(&self) -> (String, Vec<String>) {
        match self {
            ServerInstallType::Npm { package } => {
                ("npx".to_string(), vec!["-y".to_string(), package.clone()])
            }
            ServerInstallType::Uvx { package } => ("uvx".to_string(), vec![package.clone()]),
            ServerInstallType::Docker { image } => {
                ("docker".to_string(), vec!["run".to_string(), image.clone()])
            }
            ServerInstallType::Remote { url } => ("remote".to_string(), vec![url.clone()]),
        }
    }
}

impl fmt::Display for ServerInstallType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Minimal search result entry.
#[derive(Debug, Clone)]
pub struct McpServerInfo {
    pub qualified_name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub source: String,
    pub install: ServerInstallType,
    pub author: Option<String>,
    pub downloads: Option<u64>,
}

impl McpServerInfo {
    pub fn short_description(&self) -> String {
        self.description
            .as_ref()
            .map(|d| d.chars().take(96).collect())
            .unwrap_or_else(|| "-".to_string())
    }
}

/// Detailed server entry with install requirements.
#[derive(Debug, Clone)]
pub struct McpServerDetail {
    pub info: McpServerInfo,
    pub repository: Option<String>,
    pub required_env: Vec<EnvVarSpec>,
}

/// Environment variable requirement spec from registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvVarSpec {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<String>,
}
