//! Provider error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider '{0}' not found in configuration")]
    ProviderNotFound(String),

    #[error("Provider '{0}' is disabled")]
    ProviderDisabled(String),

    #[error(
        "Provider '{provider}' is not compatible with {ai_type}. Compatible AI types: {compatible}"
    )]
    IncompatibleProvider {
        provider: String,
        ai_type: String,
        compatible: String,
    },

    #[error("Failed to load provider configuration: {0}")]
    ConfigLoadError(String),

    #[error("Failed to save provider configuration: {0}")]
    ConfigSaveError(String),

    #[error("Invalid provider configuration: {0}")]
    InvalidConfig(String),

    #[error("Provider name '{0}' is reserved and cannot be modified")]
    ReservedName(String),

    #[error("Provider '{0}' already exists")]
    DuplicateProvider(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Generic error: {0}")]
    GenericError(#[from] anyhow::Error),
}

pub type ProviderResult<T> = Result<T, ProviderError>;
