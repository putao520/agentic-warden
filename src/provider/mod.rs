//! Third-party API Provider management module
//!
//! This module provides functionality to manage third-party API providers
//! and inject environment variables when launching AI CLIs.

pub mod config;
pub mod env_injector;
pub mod env_mapping;
pub mod error;
pub mod manager;

// Re-export commonly used types
pub use config::AiType;
pub use env_injector::EnvInjector;
pub use manager::ProviderManager;
