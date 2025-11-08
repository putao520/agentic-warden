//! Third-party API Provider management module
//!
//! This module provides functionality to manage third-party API providers
//! and inject environment variables when launching AI CLIs.

// pub mod commands; // Temporarily commented out - file corrupted
pub mod config;
pub mod env_injector;
pub mod env_mapping;
pub mod error;
pub mod manager;
pub mod network_detector;
pub mod recommendation_engine;
pub mod token_validator;

// Re-export commonly used types (some removed as unused)
pub use config::AiType;
pub use env_injector::EnvInjector;
pub use manager::ProviderManager;
