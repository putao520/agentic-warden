//! Validation helpers for plugin manifests.

use crate::commands::market::plugin::PluginManifest;
use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult};

pub fn validate_manifest(manifest: &PluginManifest) -> MarketResult<()> {
    if manifest.name.trim().is_empty() {
        return Err(MarketError::new(
            MarketErrorCode::McpExtractionFailed,
            "plugin.json missing required field: name",
        ));
    }
    if manifest.version.trim().is_empty() {
        return Err(MarketError::new(
            MarketErrorCode::McpExtractionFailed,
            "plugin.json missing required field: version",
        ));
    }
    if manifest.description.trim().is_empty() {
        return Err(MarketError::new(
            MarketErrorCode::McpExtractionFailed,
            "plugin.json missing required field: description",
        ));
    }
    if manifest.author.name.trim().is_empty() {
        return Err(MarketError::new(
            MarketErrorCode::McpExtractionFailed,
            "plugin.json missing required field: author.name",
        ));
    }
    Ok(())
}
