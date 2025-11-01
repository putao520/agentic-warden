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
    #[allow(dead_code)]
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

    /// Create OAuth client from environment variables
    pub fn from_env() -> Result<Self> {
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| anyhow::anyhow!("GOOGLE_CLIENT_ID environment variable not set"))?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| anyhow::anyhow!("GOOGLE_CLIENT_SECRET environment variable not set"))?;
        let refresh_token = std::env::var("GOOGLE_REFRESH_TOKEN").ok();

        Ok(Self::new(client_id, client_secret, refresh_token))
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
        assert!(auth_url.contains("urn%3Aietf%3Awg%3Aoauth%3A2.0%3Aoob") || auth_url.contains("urn:ietf:wg:oauth:2.0:oob"));
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
