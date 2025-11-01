//! Smart OAuth Authentication (OOB - Out of Band)
//!
//! OAuth authentication system using Google's OOB (Out-of-Band) flow
//! Designed specifically for CLI tools without callback servers

use anyhow::{Context, Result, anyhow};
use console::Term;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant, timeout};
use tracing::{debug, info};

use crate::sync::oauth_client::{OAuthClient, OAuthConfig, OAuthTokenResponse};

/// OAuth authentication status
#[derive(Debug, Clone)]
pub enum AuthState {
    /// Initializing
    Initializing,
    /// Waiting for manual authorization code
    WaitingForCode { url: String, expires_at: Instant },
    /// Processing
    Processing,
    /// Success
    Success,
    /// Failed
    Failed(String),
}

/// Smart OAuth authenticator using OOB flow
pub struct SmartOAuthAuthenticator {
    state: Arc<RwLock<AuthState>>,
    config: OAuthConfig,
    timeout_duration: Duration,
}

impl SmartOAuthAuthenticator {
    /// Create new authenticator
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(AuthState::Initializing)),
            config,
            timeout_duration: Duration::from_secs(300), // 5 minute timeout
        }
    }

    /// Get OAuth configuration
    pub fn get_config(&self) -> &OAuthConfig {
        &self.config
    }

    /// Execute OOB OAuth authentication
    pub async fn authenticate(&self) -> Result<OAuthTokenResponse> {
        info!("Starting OOB OAuth authentication");

        // 1. Generate OOB authorization URL
        let auth_url = self.generate_auth_url()?;

        // 2. Display OOB authorization instructions
        self.display_oob_instructions(&auth_url).await?;

        // 3. Update state
        self.update_state(AuthState::WaitingForCode {
            url: auth_url.clone(),
            expires_at: Instant::now() + self.timeout_duration,
        })
        .await;

        // 4. Wait for manual authorization code input
        let auth_code = self.wait_for_auth_code().await?;

        // 5. Exchange token
        let token_response = self.exchange_code_for_token(auth_code).await?;

        info!("OOB OAuth authentication completed successfully");
        Ok(token_response)
    }

    /// Generate OOB authorization URL
    fn generate_auth_url(&self) -> Result<String> {
        let oauth_client = OAuthClient::new(
            self.config.client_id.clone(),
            self.config.client_secret.clone(),
            None,
        );

        oauth_client
            .generate_auth_url()
            .context("Failed to generate OOB auth URL")
    }

    /// Display OOB authorization instructions
    async fn display_oob_instructions(&self, auth_url: &str) -> Result<()> {
        let term = Term::stdout();

        println!();
        println!("{}", "═".repeat(70));
        println!("🔐 GOOGLE DRIVE AUTHENTICATION (OOB FLOW)");
        println!("{}", "═".repeat(70));
        println!();
        println!("📦 What will be stored:");
        println!("   • Configuration backups (compressed archives)");
        println!("   • Sync metadata (last sync time, checksums)");
        println!();
        println!("🔒 Permissions requested:");
        println!("   • Create files and folders");
        println!("   • Read/modify files created by this app only");
        println!("   • No access to other Google Drive files");
        println!();
        println!("{}", "═".repeat(70));
        println!("🔗 AUTHENTICATION URL:");
        println!("{}", "═".repeat(70));
        println!();
        println!("🌐 Open this URL in your browser:");
        println!();

        // Display clickable URL if terminal supports it
        if term.features().colors_supported() {
            let clickable_url = format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", auth_url, auth_url);
            println!("   {}", clickable_url);
            println!();
            println!("   💡 Click the URL above to open in browser");
            println!("   📋 Or right-click to copy, then paste in your browser");
        } else {
            println!("   {}", auth_url);
            println!();
            println!("   📋 Copy the URL above and paste in your browser");
        }

        println!();
        println!("{}", "─".repeat(70));
        println!("📋 OOB (Out-of-Band) Flow Instructions:");
        println!("   1. Click the URL above (opens in browser)");
        println!("   2. Sign in to your Google account");
        println!("   3. Grant permissions to Agentic-Warden");
        println!("   4. Google will display an authorization code");
        println!("   5. Copy the authorization code");
        println!("   6. Paste the code below");
        println!("{}", "─".repeat(70));
        println!();

        // Try to open browser automatically
        if let Err(e) = open::that(auth_url) {
            println!("⚠️  Could not open browser automatically: {}", e);
            println!("   Please manually click the URL above");
        } else {
            println!("✅ Browser opened automatically");
        }

        println!();
        println!("📝 After completing authorization, Google will show you a code.");
        println!("   Copy that code and enter it below:");
        println!("{}", "═".repeat(70));
        println!();

        Ok(())
    }

    /// Wait for manual authorization code input
    async fn wait_for_auth_code(&self) -> Result<String> {
        println!("📝 Authorization Code:");

        // Use dialoguer for input with timeout handling
        let auth_code = timeout(self.timeout_duration, async {
            loop {
                match dialoguer::Input::<String>::new()
                    .with_prompt("🔑 Enter the authorization code from Google")
                    .allow_empty(false)
                    .interact_text()
                {
                    Ok(code) => {
                        let code = code.trim();
                        if !code.is_empty() {
                            info!("Received authorization code via manual input");
                            return Ok(code.to_string());
                        } else {
                            println!("❌ Authorization code cannot be empty. Please try again.");
                        }
                    }
                    Err(e) => {
                        debug!("Input error: {}", e);
                        return Err(anyhow!("Failed to read authorization code: {}", e));
                    }
                }
            }
        })
        .await;

        match auth_code {
            Ok(Ok(code)) => {
                println!("✅ Authorization code received!");
                Ok(code)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                println!("⏰ Authentication timed out after 5 minutes");
                Err(anyhow!("Authentication timed out after 5 minutes"))
            }
        }
    }

    /// Exchange authorization code for access token
    async fn exchange_code_for_token(&self, code: String) -> Result<OAuthTokenResponse> {
        self.update_state(AuthState::Processing).await;

        println!("🔄 Exchanging authorization code for access token...");

        let mut oauth_client = OAuthClient::new(
            self.config.client_id.clone(),
            self.config.client_secret.clone(),
            None,
        );

        match oauth_client.exchange_code_for_tokens(&code).await {
            Ok(token_response) => {
                self.update_state(AuthState::Success).await;
                println!("🎉 Authentication successful!");
                Ok(token_response)
            }
            Err(e) => {
                let error_msg = format!("Failed to exchange authorization code: {}", e);
                self.update_state(AuthState::Failed(error_msg.clone()))
                    .await;
                Err(anyhow!(error_msg))
            }
        }
    }

    /// Update authentication status
    async fn update_state(&self, new_state: AuthState) {
        *self.state.write().await = new_state;
    }

    /// Get current authentication status
    #[allow(dead_code)]
    pub async fn get_state(&self) -> AuthState {
        self.state.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_smart_oauth_creation() {
        let config = OAuthConfig {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            refresh_token: None,
            access_token: None,
            expires_in: 0,
            token_type: "Bearer".to_string(),
            scopes: vec!["https://www.googleapis.com/auth/drive.file".to_string()],
        };

        let authenticator = SmartOAuthAuthenticator::new(config);
        assert!(matches!(
            authenticator.get_state().await,
            AuthState::Initializing
        ));
    }
}
