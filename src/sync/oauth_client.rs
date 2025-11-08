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

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            access_token: None,
            refresh_token: None,
            expires_in: 0,
            token_type: "Bearer".to_string(),
            scopes: vec![],
        }
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

/// OAuth client for handling OOB authentication
#[derive(Debug, Clone)]
pub struct OAuthClient {
    config: OAuthConfig,
    /// Path to the file where OAuth tokens are persisted
    auth_file_path: PathBuf,
}

impl OAuthClient {
    /// Create new OAuth client with default Google Drive scopes
    pub fn new(client_id: String, client_secret: String, refresh_token: Option<String>) -> Self {
        let config = OAuthConfig {
            client_id,
            client_secret,
            refresh_token,
            scopes: vec![
                "https://www.googleapis.com/auth/drive.file".to_string(),
                "https://www.googleapis.com/auth/drive.metadata.readonly".to_string(),
            ],
            ..Default::default()
        };

        let auth_file_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("agentic-warden")
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

    /// Generate authorization URL
    pub fn generate_auth_url(&self) -> Result<String> {
        let redirect_uri = "urn:ietf:wg:oauth:2.0:oob"; // OOB flow
        let scope = self.config.scopes.join(" ");
        let auth_url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scope)
        );
        Ok(auth_url)
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code_for_tokens(&mut self, code: &str) -> Result<OAuthTokenResponse> {
        info!("Exchanging OOB authorization code for tokens...");

        let client = reqwest::Client::new();
        let params = [
            ("client_id", self.config.client_id.clone()),
            ("client_secret", self.config.client_secret.clone()),
            ("code", code.to_string()),
            ("grant_type", "authorization_code".to_string()),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob".to_string()), // OOB redirect URI
        ];

        let response = client
            .post("https://oauth2.googleapis.com/token")
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

            // Persist tokens to disk for future use
            if let Err(e) = self.save() {
                // Log error but don't fail the operation
                debug!("Warning: Failed to save OAuth tokens to disk: {}", e);
            }

            Ok(token_response)
        } else {
            let error_text = response.text().await?;
            Err(anyhow::anyhow!("OOB code exchange failed: {}", error_text))
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
            .post("https://oauth2.googleapis.com/token")
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
                debug!("Warning: Failed to save refreshed OAuth tokens to disk: {}", e);
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

    /// Load OAuth configuration from disk
    ///
    /// Attempts to load previously saved OAuth tokens from disk. Returns None if
    /// the file doesn't exist or cannot be read.
    pub fn load(client_id: String, client_secret: String) -> Result<Option<Self>> {
        let auth_file_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("agentic-warden")
            .join("auth.json");

        debug!("Attempting to load OAuth configuration from {:?}", auth_file_path);

        // Check if file exists
        if !auth_file_path.exists() {
            debug!("OAuth configuration file does not exist");
            return Ok(None);
        }

        // Read and parse the file
        let json = std::fs::read_to_string(&auth_file_path)?;
        let mut config: OAuthConfig = serde_json::from_str(&json)?;

        // Update client credentials (they may have changed)
        config.client_id = client_id;
        config.client_secret = client_secret;

        info!("OAuth configuration loaded successfully");
        Ok(Some(Self {
            config,
            auth_file_path,
        }))
    }

    /// Validate configuration
    pub fn validate_config(&self) -> Result<()> {
        if self.config.client_id.is_empty() {
            return Err(anyhow::anyhow!("Client ID is required"));
        }

        if self.config.client_secret.is_empty() {
            return Err(anyhow::anyhow!("Client secret is required"));
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
    fn test_oob_auth_url_generation() {
        let client = OAuthClient::new(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            None,
        );

        let auth_url = client.generate_auth_url().unwrap();
        // The OOB redirect URI gets URL encoded
        assert!(
            auth_url.contains("urn%3Aietf%3Awg%3Aoauth%3A2.0%3Aoob")
                || auth_url.contains("urn:ietf:wg:oauth:2.0:oob")
        );
        assert!(auth_url.contains("test_client_id"));
        assert!(!auth_url.contains("localhost"));
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
