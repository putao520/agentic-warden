//! CLI OAuth Authorization Handler Module
//!
//! Provides direct CLI-based authorization with unique authorization URLs

use crate::sync::oauth_client::{DeviceCodeResponse, OAuthConfig};
use crate::sync::smart_oauth::SmartOAuthAuthenticator;
use anyhow::Result;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::interval;

/// CLI OAuth authorization handler
pub struct CliOAuthHandler {
    authenticator: SmartOAuthAuthenticator,
}

impl CliOAuthHandler {
    /// Create new CLI OAuth handler with built-in public client
    /// As per SPEC REQ-015, uses the built-in public OAuth client without any external configuration
    pub fn new() -> Result<Self> {
        eprintln!("[EPRINTLN] CliOAuthHandler::new() START");

        eprintln!("[EPRINTLN] About to call OAuthConfig::default()");
        let config = OAuthConfig::default();
        eprintln!("[EPRINTLN] OAuthConfig::default() completed");
        println!("DEBUG: OAuthConfig::default() completed, client_id: {}",
            if config.client_id.is_empty() { "empty" } else { "set" });

        eprintln!("[EPRINTLN] About to create SmartOAuthAuthenticator");
        let authenticator = SmartOAuthAuthenticator::new(config);
        eprintln!("[EPRINTLN] SmartOAuthAuthenticator created successfully");
        println!("DEBUG: SmartOAuthAuthenticator created successfully");
        eprintln!("[EPRINTLN] CliOAuthHandler::new() END");
        Ok(Self { authenticator })
    }

    /// Start CLI device authorization flow
    pub async fn start_device_flow(&self) -> Result<()> {
        println!("🔐 Starting Google Drive authorization flow...");
        println!("DEBUG: About to call self.authenticator.start_device_flow()...");

        // Start device authorization
        let device_code = self.authenticator.start_device_flow().await?;

        println!("DEBUG: Device flow successful, got device code");

        // Display authorization information
        self.display_auth_info(&device_code).await?;

        // Wait for user authorization
        self.wait_for_authorization(&device_code).await?;

        println!("✅ Authorization completed! You can now perform sync operations.");
        Ok(())
    }

    /// Display authorization information with URL
    async fn display_auth_info(&self, device_code: &DeviceCodeResponse) -> Result<()> {
        println!("\n{}", "=".to_string().repeat(60));
        println!("📱 Please complete authorization in your browser");
        println!("{}", "=".to_string().repeat(60));

        // Display authorization URL
        println!("\n🔗 Authorization URL (complete with embedded parameters):");
        println!("{}", device_code.verification_url_complete);

        // Display expiration time
        let expires_minutes = device_code.expires_in / 60;
        let expires_seconds = device_code.expires_in % 60;
        println!("\n⏰ Authorization code will expire in {} min {} sec", expires_minutes, expires_seconds);

        println!("\nPress Enter when you've completed authorization...");
        io::stdout().flush()?;

        // Wait for user input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        Ok(())
    }


    /// Wait for user authorization to complete
    async fn wait_for_authorization(&self, device_code: &DeviceCodeResponse) -> Result<()> {
        let interval_seconds = device_code.interval;
        let max_attempts = device_code.expires_in / interval_seconds;

        println!("\n⏳ Waiting for authorization completion...");

        // Create timer with specified interval for polling
        let mut timer = interval(Duration::from_secs(interval_seconds));

        for attempt in 1..=max_attempts {
            // Check if authorized
            match self.authenticator.get_state().await {
                crate::sync::smart_oauth::AuthState::Authenticated { .. } => {
                    println!("✅ Authorization successful!");
                    return Ok(());
                }
                crate::sync::smart_oauth::AuthState::Error { message } => {
                    return Err(anyhow::anyhow!("Authorization failed: {}", message));
                }
                _ => {
                    // Continue waiting
                }
            }

            // Wait for interval
            timer.tick().await;

            // Show waiting status
            if attempt % 5 == 0 {
                let remaining_minutes = (max_attempts - attempt) * interval_seconds / 60;
                let remaining_seconds = ((max_attempts - attempt) * interval_seconds) % 60;
                println!("⏳ Continuing to wait for authorization... (approx {}:{:02} remaining)",
                    remaining_minutes, remaining_seconds);
            }
        }

        Err(anyhow::anyhow!("Authorization timeout, please try again"))
    }
}

/// Handle CLI authorization flow entry point
pub async fn handle_cli_auth() -> Result<()> {
    eprintln!("[EPRINTLN] DEBUG: handle_cli_auth() ENTRY POINT");
    println!("DEBUG: handle_cli_auth() called");
    println!("DEBUG: Creating CliOAuthHandler with built-in public client...");
    eprintln!("[EPRINTLN] DEBUG: About to create CliOAuthHandler");

    let handler = CliOAuthHandler::new()?;
    println!("DEBUG: CliOAuthHandler created successfully");

    // Show warning if using public client
    println!("DEBUG: Getting warning message from authenticator");
    if let Some(warning) = handler.authenticator.get_warning_message_async().await {
        println!("⚠️  {}", warning);
        println!();
    }

    println!("DEBUG: About to start device flow");
    handler.start_device_flow().await
}

