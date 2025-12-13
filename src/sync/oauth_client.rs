use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
    pub scopes: Vec<String>,
}

impl OAuthConfig {
    /// Create a new OAuth configuration with custom client credentials
    pub fn new_custom(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            access_token: None,
            refresh_token: None,
            expires_in: 0,
            token_type: "Bearer".to_string(),
            scopes: vec![
                "https://www.googleapis.com/auth/drive.file".to_string(),
                "https://www.googleapis.com/auth/drive.metadata.readonly".to_string(),
            ],
        }
    }

    /// Create OAuth configuration for Google Drive
    /// Uses the built-in public OAuth client as specified in SPEC REQ-015
    pub fn for_google_drive() -> Self {
        // Built-in public client credentials from SPEC REQ-015
        // These are public client IDs that Google has designated for public OAuth flows
        Self::new_custom(
            "77185425430.apps.googleusercontent.com".to_string(),
            "GOCSPX-1r0aNJW8XY1Mqg4k5L_KzQDGH43".to_string(),
        )
    }

    /// Check if this configuration has valid credentials
    pub fn has_valid_credentials(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }

    /// Get configuration warning message for invalid credentials
    pub fn get_warning_message(&self) -> Option<String> {
        if self.client_id.is_empty() || self.client_secret.is_empty() {
            Some("❌ Google OAuth client credentials are required. Please create your own Google OAuth credentials:".to_string())
        } else if self.client_id == "77185425430.apps.googleusercontent.com" {
            Some("⚠️  Using public OAuth client. It may have limitations. For better results, create your own Google OAuth credentials in Google Cloud Console.".to_string())
        } else {
            None
        }
    }
}

impl Default for OAuthConfig {
    fn default() -> Self {
        println!("DEBUG: OAuthConfig::default() called");
        println!("DEBUG: Calling Self::for_google_drive()");
        let result = Self::for_google_drive();
        println!("DEBUG: Self::for_google_drive() completed successfully");
        result
    }
}

/// OAuth token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
    pub scope: Option<String>,
}

/// Device code response from Google OAuth 2.0 Device Flow (RFC 8628)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub verification_url_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// OAuth client for handling Device Flow (RFC 8628) and OOB authentication
#[derive(Debug, Clone)]
pub struct OAuthClient {
    config: OAuthConfig,
    /// Path to the file where OAuth tokens are persisted
    auth_file_path: PathBuf,
}

impl OAuthClient {
    /// Create new OAuth client with default Google Drive scopes
    pub fn new(client_id: String, client_secret: String, refresh_token: Option<String>) -> Self {
        // Check for custom credentials from environment variables
        let custom_client_id = std::env::var("AIW_OAUTH_CLIENT_ID").ok();
        let custom_client_secret = std::env::var("AIW_OAUTH_CLIENT_SECRET").ok();

        let final_client_id = custom_client_id.unwrap_or(client_id);
        let final_client_secret = custom_client_secret.unwrap_or(client_secret);

        let config = OAuthConfig {
            client_id: final_client_id,
            client_secret: final_client_secret,
            refresh_token,
            scopes: vec![
                "https://www.googleapis.com/auth/drive.file".to_string(),
                "https://www.googleapis.com/auth/drive.metadata.readonly".to_string(),
            ],
            ..Default::default()
        };

        let auth_file_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("aiw")
            .join("auth.json");

        Self {
            config,
            auth_file_path,
        }
    }

    /// Create OAuth client with scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.config.scopes = scopes;
        self
    }

    /// Start Device Flow (RFC 8628) - Request device and user codes
    /// Returns device code response with user_code and verification_url to show to user
    pub async fn start_device_flow(&self) -> Result<DeviceCodeResponse> {
        info!("Starting Device Flow (RFC 8628) authorization...");
        info!("Using client ID: {}", self.config.client_id);
        info!("Using scopes: {:?}", self.config.scopes);

        let client = reqwest::Client::new();
        let scope = self.config.scopes.join(" ");

        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("scope", scope.as_str()),
        ];

        info!("Sending request to Google OAuth endpoint...");

        // Add timeout for network requests
        let timeout_duration = std::time::Duration::from_secs(15);
        let response = client
            .post("https://accounts.google.com/o/oauth2/device/code")
            .form(&params)
            .timeout(timeout_duration)
            .send()
            .await?;

        info!("Response status: {}", response.status());

        if response.status().is_success() {
            let mut device_response: DeviceCodeResponse = response.json().await?;
            // 生成完整的授权 URL，包含用户码和设备码
            device_response.verification_url_complete = format!(
                "{}?user_code={}&device_code={}",
                device_response.verification_url,
                device_response.user_code,
                device_response.device_code
            );
            info!(
                "Device code obtained. User code: {}",
                device_response.user_code
            );
            Ok(device_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            eprintln!("❌ Device flow initialization failed: {}", error_text);

            // Check if it's an invalid client error
            if error_text.contains("invalid_client") {
                eprintln!("💡 The built-in public OAuth client is no longer valid.");
                eprintln!("💡 Please create your own Google OAuth credentials:");
                eprintln!("   1. Go to https://console.cloud.google.com/");
                eprintln!("   2. Create a new OAuth client ID with 'Desktop app' type");
                eprintln!("   3. Note down the client ID and client secret");
                eprintln!("   4. Set environment variables:");
                eprintln!("      export AIW_OAUTH_CLIENT_ID=your_client_id");
                eprintln!("      export AIW_OAUTH_CLIENT_SECRET=your_client_secret");
                return Err(anyhow::anyhow!("Public OAuth client is no longer valid. Please create your own Google OAuth credentials."));
            }

            Err(anyhow::anyhow!("Device flow initialization failed: {}", error_text))
        }
    }

    /// Poll for tokens using device code
    /// Should be called repeatedly with the interval specified in DeviceCodeResponse
    /// Returns Ok(Some(tokens)) when user completes authorization
    /// Returns Ok(None) when still waiting (authorization_pending)
    /// Returns Err when polling fails or user denies
    pub async fn poll_for_tokens(
        &mut self,
        device_code: &str,
    ) -> Result<Option<OAuthTokenResponse>> {
        debug!("Polling for device flow authorization...");

        let client = reqwest::Client::new();
        let params = [
            ("client_id", self.config.client_id.clone()),
            ("client_secret", self.config.client_secret.clone()),
            ("device_code", device_code.to_string()),
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            ),
        ];

        let response = client
            .post("https://accounts.google.com/o/oauth2/token")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: OAuthTokenResponse = response.json().await?;

            // Update the config with new tokens
            self.config.access_token = Some(token_response.access_token.clone());
            if token_response.refresh_token.is_some() {
                self.config.refresh_token = token_response.refresh_token.clone();
            }
            self.config.expires_in = token_response.expires_in;

            // Persist tokens to disk
            if let Err(e) = self.save() {
                debug!("Warning: Failed to save OAuth tokens to disk: {}", e);
            }

            info!("Device flow authorization completed successfully");
            Ok(Some(token_response))
        } else {
            let status = response.status();
            let error_body: Result<serde_json::Value, _> = response.json().await;

            if let Ok(error_json) = error_body {
                if let Some(error_type) = error_json.get("error").and_then(|e| e.as_str()) {
                    match error_type {
                        "authorization_pending" => {
                            // User hasn't completed authorization yet - this is expected
                            debug!("Authorization still pending...");
                            return Ok(None);
                        }
                        "slow_down" => {
                            // We're polling too fast - should increase interval
                            debug!("Polling too fast, should slow down");
                            return Ok(None);
                        }
                        "access_denied" => {
                            return Err(anyhow::anyhow!("User denied authorization"));
                        }
                        "expired_token" => {
                            return Err(anyhow::anyhow!(
                                "Device code expired, please restart authorization"
                            ));
                        }
                        _ => {
                            return Err(anyhow::anyhow!("Device flow error: {}", error_type));
                        }
                    }
                }
            }

            Err(anyhow::anyhow!(
                "Device flow polling failed with status: {}",
                status
            ))
        }
    }

    /// Get authenticated access token
    pub async fn access_token(&mut self) -> Result<String> {
        // If we have a refresh token, try to use it
        if let Some(_refresh_token) = &self.config.refresh_token {
            match self.refresh_access_token().await {
                Ok(response) => {
                    return Ok(response.access_token);
                }
                Err(e) => {
                    debug!("Token refresh failed: {}, trying code exchange", e);
                }
            }
        }

        Err(anyhow::anyhow!("No valid authentication tokens available"))
    }

    /// Refresh access token
    pub async fn refresh_access_token(&mut self) -> Result<OAuthTokenResponse> {
        info!("Refreshing OAuth access token...");

        let refresh_token = self
            .config
            .refresh_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No refresh token available"))?;

        let client = reqwest::Client::new();
        let params = [
            ("client_id", self.config.client_id.clone()),
            ("client_secret", self.config.client_secret.clone()),
            ("refresh_token", refresh_token.clone()),
            ("grant_type", "refresh_token".to_string()),
        ];

        let response = client
            .post("https://accounts.google.com/o/oauth2/token")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: OAuthTokenResponse = response.json().await?;

            // Update the config with new tokens
            self.config.access_token = Some(token_response.access_token.clone());
            if token_response.refresh_token.is_some() {
                self.config.refresh_token = token_response.refresh_token.clone();
            }
            self.config.expires_in = token_response.expires_in;

            // Persist refreshed tokens to disk
            if let Err(e) = self.save() {
                // Log error but don't fail the operation
                debug!(
                    "Warning: Failed to save refreshed OAuth tokens to disk: {}",
                    e
                );
            }

            Ok(token_response)
        } else {
            let error_text = response.text().await?;
            Err(anyhow::anyhow!("Token refresh failed: {}", error_text))
        }
    }

    /// Get configuration reference
    pub fn config(&self) -> &OAuthConfig {
        &self.config
    }

    /// Check if client is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.config.access_token.is_some() || self.config.refresh_token.is_some()
    }

    /// Save OAuth configuration to disk
    ///
    /// Persists the OAuth tokens and configuration to a JSON file in the user's
    /// config directory. This allows tokens to be reused across application restarts.
    pub fn save(&self) -> Result<()> {
        debug!("Saving OAuth configuration to {:?}", self.auth_file_path);

        // Ensure the parent directory exists
        if let Some(parent) = self.auth_file_path.parent() {
            std::fs::create_dir_all(parent)?;

            // Set restrictive permissions on Unix systems (only user can access)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(parent)?.permissions();
                perms.set_mode(0o700); // rwx------
                std::fs::set_permissions(parent, perms)?;
            }
        }

        // Serialize configuration to JSON
        let json = serde_json::to_string_pretty(&self.config)?;

        // Write to file
        std::fs::write(&self.auth_file_path, json)?;

        // Set restrictive permissions on the file (only user can read/write)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.auth_file_path)?.permissions();
            perms.set_mode(0o600); // rw-------
            std::fs::set_permissions(&self.auth_file_path, perms)?;
        }

        info!("OAuth configuration saved successfully");
        Ok(())
    }

    /// Validate configuration
    pub fn validate_config(&self) -> Result<()> {
        if self.config.client_id.is_empty() {
            return Err(anyhow::anyhow!("Client ID is required. Please create Google OAuth credentials or use environment variables AIW_OAUTH_CLIENT_ID and AIW_OAUTH_CLIENT_SECRET."));
        }

        if self.config.client_secret.is_empty() {
            return Err(anyhow::anyhow!("Client secret is required. Please create Google OAuth credentials or use environment variables AIW_OAUTH_CLIENT_ID and AIW_OAUTH_CLIENT_SECRET."));
        }

        if self.config.scopes.is_empty() {
            return Err(anyhow::anyhow!("At least one scope is required"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config_default() {
        let config = OAuthConfig::default();
        assert_eq!(config.token_type, "Bearer");
        assert!(config.access_token.is_none());
        assert!(config.refresh_token.is_none());
    }

    #[test]
    fn test_oauth_client_creation() {
        let client = OAuthClient::new(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            Some("test_refresh_token".to_string()),
        );

        assert_eq!(client.config().client_id, "test_client_id");
        assert_eq!(client.config().client_secret, "test_client_secret");
        assert_eq!(
            client.config().refresh_token,
            Some("test_refresh_token".to_string())
        );
    }

    #[test]
    fn test_config_validation() {
        // Test valid client with default scopes
        let valid_client = OAuthClient::new(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            None,
        );
        assert!(valid_client.validate_config().is_ok());

        // Test invalid client - empty client_id
        let invalid_client1 =
            OAuthClient::new("".to_string(), "test_client_secret".to_string(), None);
        assert!(invalid_client1.validate_config().is_err());

        // Test invalid client - empty client_secret
        let invalid_client2 = OAuthClient::new("test_client_id".to_string(), "".to_string(), None);
        assert!(invalid_client2.validate_config().is_err());

        // Test invalid client - no scopes (using with_scopes to override defaults)
        let invalid_client3 = OAuthClient::new(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            None,
        )
        .with_scopes(vec![]);
        assert!(invalid_client3.validate_config().is_err());
    }
}
