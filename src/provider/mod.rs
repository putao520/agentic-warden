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

// Re-export commonly used types
pub use config::{AiType, Provider, RecommendationScenario, SupportMode};
pub use env_injector::EnvInjector;
pub use manager::ProviderManager;
pub use network_detector::{NetworkDetector, NetworkStatus};
pub use recommendation_engine::{RecommendationEngine, RecommendationPreferences};
pub use token_validator::{TokenValidationResult, TokenValidator, ValidationStatus};
