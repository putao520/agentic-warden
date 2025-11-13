//! Memory configuration module

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub ollama_url: String,
    pub qdrant_url: String,
    pub embedding_model: String,
    pub llm_model: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            qdrant_url: "http://localhost:26333".to_string(),
            embedding_model: "qwen3-embedding:0.6b".to_string(),
            llm_model: "qwen3:8b".to_string(),
        }
    }
}

impl MemoryConfig {
    pub fn load_from_provider_config() -> anyhow::Result<Self> {
        // Load using the proper providers config system
        match crate::provider::config::ProvidersConfig::load() {
            Ok(providers_config) => Ok(providers_config.get_memory_config()),
            Err(_) => Ok(Self::default()),
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate URLs
        if self.ollama_url.is_empty() {
            return Err(anyhow::anyhow!("ollama_url cannot be empty"));
        }

        if self.qdrant_url.is_empty() {
            return Err(anyhow::anyhow!("qdrant_url cannot be empty"));
        }

        // Validate models
        if self.embedding_model.is_empty() {
            return Err(anyhow::anyhow!("embedding_model cannot be empty"));
        }

        if self.llm_model.is_empty() {
            return Err(anyhow::anyhow!("llm_model cannot be empty"));
        }

        Ok(())
    }
}
