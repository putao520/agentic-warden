//! Plugin marketplace module.

pub mod cache;
pub mod cli;
pub mod cli_marketplace;
pub mod cli_plugins;
pub mod cli_utils;
pub mod config;
pub mod config_utils;
pub mod filter;
pub mod github_source;
pub mod installer;
pub mod local_source;
pub mod plugin;
pub mod plugin_io;
pub mod remote_source;
pub mod source;
pub mod validator;

pub use cli::handle_plugin_action;
pub use source::{MarketError, MarketErrorCode, MarketSource, MarketplaceSourceConfig};
