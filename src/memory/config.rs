use anyhow::{anyhow, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_FASTEMBED_MODEL: &str = "BGESmallENV15";
const DEFAULT_LLM_ENDPOINT: &str = "http://localhost:11434";
const DEFAULT_LLM_MODEL: &str = "qwen2.5:7b";
/// Configuration for the dual-mode memory subsystem.
///
/// The structure intentionally keeps the fields minimal—embeddings are powered
/// by FastEmbed, routing uses MemVDB, and conversation history relies on
/// SahomeDB with a fixed path to guarantee deterministic behaviour.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MemoryConfig {
    /// Identifier of the FastEmbed model to use.
    pub fastembed_model: String,
    /// Ollama endpoint for the internal LLM (decision engine only).
    pub llm_endpoint: String,
    /// Model name to pass to Ollama.
    pub llm_model: String,
    /// Absolute path to the SahomeDB file.
    pub sahome_db_path: PathBuf,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            fastembed_model: std::env::var("AGENTIC_WARDEN_FASTEMBED_MODEL")
                .unwrap_or_else(|_| DEFAULT_FASTEMBED_MODEL.to_string()),
            llm_endpoint: std::env::var("AGENTIC_WARDEN_LLM_ENDPOINT")
                .unwrap_or_else(|_| DEFAULT_LLM_ENDPOINT.to_string()),
            llm_model: std::env::var("AGENTIC_WARDEN_LLM_MODEL")
                .unwrap_or_else(|_| DEFAULT_LLM_MODEL.to_string()),
            sahome_db_path: default_history_db_path(),
        }
    }
}

impl MemoryConfig {
    /// Loads the configuration that is embedded inside the providers config file.
    pub fn load_from_provider_config() -> Result<Self> {
        use crate::provider::manager::ProviderManager;
        match ProviderManager::new() {
            Ok(manager) => Ok(manager.get_providers_config().memory.clone().unwrap_or_default()),
            Err(_) => Ok(Self::default()),
        }
    }

    /// Validates the configuration to guard against silent fallbacks.
    pub fn validate(&self) -> Result<()> {
        if self.fastembed_model.trim().is_empty() {
            return Err(anyhow!("fastembed_model cannot be empty"));
        }
        if self.llm_endpoint.trim().is_empty() {
            return Err(anyhow!("llm_endpoint cannot be empty"));
        }
        if self.llm_model.trim().is_empty() {
            return Err(anyhow!("llm_model cannot be empty"));
        }
        if self.sahome_db_path.as_os_str().is_empty() {
            return Err(anyhow!("sahome_db_path cannot be empty"));
        }
        if !self.sahome_db_path.is_absolute() {
            return Err(anyhow!(
                "sahome_db_path must be absolute, got {}",
                self.sahome_db_path.display()
            ));
        }
        Ok(())
    }

    /// Returns the parent directory for the SahomeDB path.
    pub fn history_directory(&self) -> PathBuf {
        self.sahome_db_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(default_history_db_dir)
    }
}

fn default_history_db_path() -> PathBuf {
    let mut base = default_history_db_dir();
    base.push("sahome.db");
    base
}

fn default_history_db_dir() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".aiw/history")
}
