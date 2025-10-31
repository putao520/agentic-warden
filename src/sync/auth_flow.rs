//! Authentication flow helper for sync operations

use super::error::{SyncError, SyncResult};
use std::path::PathBuf;

/// Check if user is authenticated with Google Drive
pub fn is_authenticated() -> SyncResult<bool> {
    let auth_path = get_auth_file_path()?;

    if !auth_path.exists() {
        return Ok(false);
    }

    let auth_content = std::fs::read_to_string(&auth_path)?;
    let auth_state: serde_json::Value = serde_json::from_str(&auth_content)?;

    let authenticated = auth_state
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    Ok(authenticated)
}

/// Get the authentication file path
pub fn get_auth_file_path() -> SyncResult<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| SyncError::ConfigError("Cannot find home directory".to_string()))?;

    Ok(home.join(".agentic-warden").join("auth.json"))
}

/// Ensure Google Drive authentication is available
/// Returns true if authenticated, false if user cancelled
pub async fn ensure_google_drive_auth() -> SyncResult<bool> {
    if is_authenticated()? {
        return Ok(true);
    }

    // For now, return an error indicating authentication is required
    // The TUI integration will be added later
    Err(SyncError::AuthenticationRequired)
}
