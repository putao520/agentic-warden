//! Smart OAuth Authentication
//!
//! Smart OAuth authentication system with concurrent callback and manual input support
//! Provides user experience similar to CODEX CLI and Claude Code

use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
};
use axum_server::Server;
use console::Term;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::{Duration, Instant, timeout};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};

use crate::sync::oauth_client::{OAuthClient, OAuthConfig, OAuthTokenResponse};

/// Authorization code source
#[derive(Debug, Clone)]
enum AuthCodeSource {
    /// From local callback server
    Callback(String),
    /// From user manual input
    Manual(String),
    /// Timeout
    Timeout,
    /// User cancelled
    Cancelled,
}

/// OAuth authentication status
#[derive(Debug, Clone)]
pub enum AuthState {
    /// Initializing
    Initializing,
    /// Waiting for callback
    WaitingForCallback {
        #[allow(dead_code)] // TODO: For UI authentication URL display
        url: String,
        #[allow(dead_code)] // TODO: For checking authentication expiration
        expires_at: Instant,
        #[allow(dead_code)] // TODO: For deciding if user manual operation prompt is needed
        has_callback_server: bool,
    },
    /// Processing
    Processing,
    /// Success
    Success,
    /// Failed
    Failed(#[allow(dead_code)] String),
}

/// Smart OAuth authenticator
#[derive(Clone)]
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

    /// Execute smart OAuth authentication
    pub async fn authenticate(&self) -> Result<OAuthTokenResponse> {
        info!("Starting smart OAuth authentication");

        // 1. Setup callback server (if possible)
        let callback_setup = self.setup_callback_server().await;
        let has_callback = callback_setup.is_ok();

        // 2. Generate authorization URL
        let auth_url = match &callback_setup {
            Ok((_, callback_url)) => self.generate_auth_url_with_callback(callback_url)?,
            Err(_) => self.generate_manual_auth_url()?,
        };

        // 3. Display authorization instructions
        self.display_auth_instructions(&auth_url, has_callback)
            .await?;

        // 4. Update state
        self.update_state(AuthState::WaitingForCallback {
            url: auth_url.clone(),
            expires_at: Instant::now() + self.timeout_duration,
            has_callback_server: has_callback,
        })
        .await;

        // 5. Run concurrent authentication flow
        let auth_code = self.run_concurrent_auth(callback_setup).await?;

        // 6. Exchange token
        let token_response = self.exchange_code_for_token(auth_code).await?;

        info!("Smart OAuth authentication completed successfully");
        Ok(token_response)
    }

    /// Setup local callback server (enhanced version with better environment adaptation)
    async fn setup_callback_server(&self) -> Result<(CallbackServerHandle, String)> {
        debug!("Attempting to setup local callback server with enhanced environment detection");

        // Detect environment type
        let is_headless = self.detect_headless_environment().await;
        let is_server = self.detect_server_environment().await;

        if is_headless {
            info!("Headless environment detected, skipping callback server");
            return Err(anyhow!(
                "Headless environment, callback server not suitable"
            ));
        }

        if is_server {
            warn!("Server environment detected, will try limited port range");
        }

        // Smarter port selection strategy
        let port_ranges = if is_server {
            // Server environment: use higher port ranges to avoid conflicts
            vec![(18080..18090), (28080..28090), (38080..38090)]
        } else {
            // Regular PC environment: prioritize common ports
            vec![(8080..8090), (18080..18090), (28080..28090)]
        };

        for range in port_ranges {
            for port in range {
                match CallbackServer::bind(port).await {
                    Ok(server) => {
                        let callback_url = format!("http://localhost:{}/callback", port);
                        let handle = CallbackServerHandle::new(server);
                        info!(
                            "Callback server successfully started on port {} in {} mode",
                            port,
                            if is_server { "server" } else { "desktop" }
                        );
                        return Ok((handle, callback_url));
                    }
                    Err(e) => {
                        debug!("Failed to bind to port {}: {}", port, e);
                        continue;
                    }
                }
            }
        }

        warn!("Could not start callback server after trying all port ranges");
        Err(anyhow!("No available ports for callback server"))
    }

    /// Detect if running in headless environment
    async fn detect_headless_environment(&self) -> bool {
        // Check DISPLAY environment variable (Unix/Linux)
        std::env::var("DISPLAY").is_err()
            && std::env::var("WAYLAND_DISPLAY").is_err()
            && std::env::var("TERM").is_ok()
    }

    /// Detect if running in server environment
    async fn detect_server_environment(&self) -> bool {
        // Check common server environment indicators
        std::env::var("SSH_CONNECTION").is_ok() ||  // SSH connection
        std::env::var("SSH_CLIENT").is_ok() ||        // SSH client
        std::env::var("SSH_TTY").is_ok() ||           // SSH terminal
        std::env::var("TERM_PROGRAM").is_err() ||     // No terminal program
        std::env::var("DESKTOP_SESSION").is_err() ||  // No desktop session
        std::env::var("XDG_CURRENT_DESKTOP").is_err() // No graphical desktop
    }

    /// Generate authorization URL with callback
    fn generate_auth_url_with_callback(&self, callback_url: &str) -> Result<String> {
        let oauth_client = OAuthClient::new(
            self.config.client_id.clone(),
            self.config.client_secret.clone(),
            None,
        );

        oauth_client
            .generate_auth_url()
            .map(|mut url| {
                // Replace redirect_uri with local callback URL
                url = url.replace("urn:ietf:wg:oauth:2.0:oob", callback_url);
                url
            })
            .context("Failed to generate auth URL with callback")
    }

    /// Generate manual authorization URL
    fn generate_manual_auth_url(&self) -> Result<String> {
        let oauth_client = OAuthClient::new(
            self.config.client_id.clone(),
            self.config.client_secret.clone(),
            None,
        );

        oauth_client
            .generate_auth_url()
            .context("Failed to generate manual auth URL")
    }

    /// Display authorization instructions
    async fn display_auth_instructions(&self, auth_url: &str, has_callback: bool) -> Result<()> {
        // Detect environment type
        let is_headless = self.detect_headless_environment().await;
        let is_server = self.detect_server_environment().await;

        println!();
        println!("{}", "═".repeat(70));

        if is_headless {
            println!("🖥️  HEADLESS/SERVER ENVIRONMENT DETECTED");
            println!();
            println!("⚠️  No browser available. You need to:");
            println!("   1. Copy the URL below");
            println!("   2. Open it on another device with browser");
            println!("   3. Complete authorization");
            println!("   4. Copy the authorization code back here");
            println!();
        } else if is_server {
            println!("🖥️  SERVER ENVIRONMENT DETECTED");
            println!();
            println!("💡 Running on server. Automatic callback may not work:");
            println!("   - If port is blocked: use manual authorization code");
            println!("   - If port is available: automatic callback will work");
            println!();
        } else {
            println!("🖥️  DESKTOP ENVIRONMENT DETECTED");
            println!();
            println!("🌐 Automatic callback enabled - just click the link!");
            println!();
        }

        println!("{}", "═".repeat(70));
        println!("🔐 GOOGLE DRIVE AUTHORIZATION");
        println!("{}", "═".repeat(70));
        println!();
        println!("⚠️  You are about to grant Agentic-Warden access to your Google Drive.");
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
        println!();

        println!("🔗 Authorization URL:");
        println!();

        let term = Term::stdout();

        if term.features().colors_supported() {
            let clickable_url = format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", auth_url, auth_url);
            println!("   {}", clickable_url);
            println!();
            println!("   💡 Tip: Click the URL above to open in browser");
            println!("   📋 Or right-click to copy, then paste in your browser");
        } else {
            println!("   {}", auth_url);
            println!();
            println!("   📋 Copy the URL above and paste in your browser");
        }

        println!();

        if is_headless {
            println!("   ⚠️  No browser detected - copy URL and open on another device");
        } else if is_server {
            println!("   💡 On SSH: Copy URL with mouse selection or Ctrl+Shift+C");
        } else {
            println!("   💡 Click to open, or right-click to copy");
        }

        println!();
        println!("{}", "─".repeat(70));
        println!("📝 How to copy the URL:");
        println!(
            "   • Most terminals: Select text with mouse, then Ctrl+Shift+C (Linux/Windows) or Cmd+C (Mac)"
        );
        println!("   • Windows Terminal: Right-click on selected text");
        println!("   • iTerm2/Terminal.app: Cmd+C after selection");
        println!("{}", "─".repeat(70));
        println!();

        // Smart browser opening strategy
        if !is_headless && !is_server {
            // Desktop environment: try to open browser automatically
            if let Err(e) = open::that(auth_url) {
                println!("⚠️  Could not open browser automatically: {}", e);
                println!("   Please manually click the URL above");
            } else {
                println!("✅ Browser opened automatically");
            }
        } else if is_server && has_callback {
            println!("🌐 Automatic callback configured if port is accessible");
        }

        println!();

        if has_callback {
            if is_headless {
                println!("📝 After authorizing, enter the authorization code below:");
            } else if is_server {
                println!("🔄 Automatic callback will work if port is accessible");
                println!("📝 If callback fails, you can manually enter the authorization code:");
            } else {
                println!("🔄 Automatic callback will handle the authorization!");
                println!("📝 Alternatively, you can manually enter the authorization code:");
            }
        } else if is_headless || is_server {
            println!("📝 Please complete authorization and enter the code below:");
        } else {
            println!("📝 After authorizing, please enter the authorization code below:");
        }

        println!("{}", "═".repeat(70));
        println!();

        Ok(())
    }

    /// Run concurrent authentication flow
    async fn run_concurrent_auth(
        &self,
        callback_setup: Result<(CallbackServerHandle, String)>,
    ) -> Result<String> {
        let (code_tx, mut code_rx) = mpsc::channel::<AuthCodeSource>(1);
        let mut tasks = tokio::task::JoinSet::new();

        // Task 1: Listen to callback server (if available)
        // First check if there's a callback server
        let has_callback = callback_setup.is_ok();

        // Task 1: Listen to callback server (if available)
        if let Ok((server_handle, _)) = callback_setup {
            let code_tx_clone = code_tx.clone();
            tasks.spawn(async move {
                debug!("Starting callback server listener");
                match server_handle.wait_for_callback().await {
                    Ok(code) => {
                        info!("Received authorization code via callback");
                        let _ = code_tx_clone.send(AuthCodeSource::Callback(code)).await;
                    }
                    Err(e) => {
                        debug!("Callback server error: {}", e);
                    }
                }
            });
        }

        // Task 2: Listen for user input (always available)
        let code_tx_clone = code_tx.clone();
        tasks.spawn(async move {
            debug!("Starting user input listener");

            let prompt = if has_callback {
                "📝 Authorization Code (or wait for automatic callback)"
            } else {
                "📝 Authorization Code"
            };

            loop {
                match dialoguer::Input::<String>::new()
                    .with_prompt(prompt)
                    .allow_empty(true)
                    .interact_text()
                {
                    Ok(code) => {
                        let code = code.trim();
                        if !code.is_empty() {
                            info!("Received authorization code via manual input");
                            let _ = code_tx_clone
                                .send(AuthCodeSource::Manual(code.to_string()))
                                .await;
                            break;
                        }
                        // Empty input, continue waiting
                    }
                    Err(e) => {
                        debug!("Input error: {}", e);
                        break;
                    }
                }
            }
        });

        // Task 3: Timeout handling
        let timeout_duration = self.timeout_duration;
        let code_tx_clone = code_tx.clone();
        tasks.spawn(async move {
            timeout(timeout_duration, std::future::pending::<()>())
                .await
                .ok();

            warn!("Authentication timed out");
            let _ = code_tx_clone.send(AuthCodeSource::Timeout).await;
        });

        // Task 4: Cancel handling (Ctrl+C)
        let code_tx_clone = code_tx.clone();
        tasks.spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Authentication cancelled by user");
            let _ = code_tx_clone.send(AuthCodeSource::Cancelled).await;
        });

        // Wait for first authorization code or signal
        match code_rx.recv().await {
            Some(AuthCodeSource::Callback(code)) => {
                println!("✅ Automatic callback received!");
                Ok(code)
            }
            Some(AuthCodeSource::Manual(code)) => {
                println!("✅ Manual code entered!");
                Ok(code)
            }
            Some(AuthCodeSource::Timeout) => {
                Err(anyhow!("Authentication timed out after 5 minutes"))
            }
            Some(AuthCodeSource::Cancelled) => Err(anyhow!("Authentication cancelled by user")),
            None => Err(anyhow!("No authorization code received")),
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
    #[allow(dead_code)] // TODO: For UI status query and debugging
    pub async fn get_state(&self) -> AuthState {
        self.state.read().await.clone()
    }
}

/// Callback server handle
struct CallbackServerHandle {
    server: CallbackServer,
}

impl CallbackServerHandle {
    fn new(server: CallbackServer) -> Self {
        Self { server }
    }

    async fn wait_for_callback(self) -> Result<String> {
        self.server.wait_for_callback().await
    }
}

/// Local callback server implementation
struct CallbackServer {
    #[allow(dead_code)]
    port: u16,
    auth_code_rx: Option<mpsc::Receiver<String>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl CallbackServer {
    /// Bind specified port and start callback server
    async fn bind(port: u16) -> Result<Self> {
        debug!("Attempting to start callback server on port {}", port);

        // Check if port is available
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        // Create channel for receiving authorization codes
        let (auth_code_tx, auth_code_rx) = mpsc::channel::<String>(1);
        let (shutdown_tx, _shutdown_rx) = oneshot::channel::<()>();

        // Create Axum application
        let app = Router::new()
            .route("/callback", get(callback_handler))
            .route("/", get(root_handler))
            .with_state(auth_code_tx)
            .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

        // Start server in background
        let _server_handle = tokio::spawn(async move {
            // Simplified implementation, not using graceful_shutdown
            match Server::bind(addr).serve(app.into_make_service()).await {
                Ok(_) => {
                    debug!("Callback server completed successfully");
                }
                Err(e) => {
                    error!("Callback server error: {}", e);
                }
            }
        });

        // Ensure server starts successfully
        tokio::time::sleep(Duration::from_millis(100)).await;

        debug!("Callback server successfully started on port {}", port);

        Ok(Self {
            port,
            auth_code_rx: Some(auth_code_rx),
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Wait for callback authorization code
    async fn wait_for_callback(mut self) -> Result<String> {
        if let Some(mut auth_code_rx) = self.auth_code_rx.take() {
            match timeout(Duration::from_secs(300), auth_code_rx.recv()).await {
                Ok(Some(code)) => {
                    info!("Received authorization code via callback");
                    Ok(code)
                }
                Ok(None) => Err(anyhow!("Callback server closed without receiving code")),
                Err(_) => Err(anyhow!("Callback timeout after 5 minutes")),
            }
        } else {
            Err(anyhow!("Callback receiver not available"))
        }
    }

    /// Shutdown server
    #[allow(dead_code)]
    async fn shutdown(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}

impl Drop for CallbackServer {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}

/// OAuth callback handler
async fn callback_handler(
    Query(params): Query<HashMap<String, String>>,
    State(auth_code_tx): State<mpsc::Sender<String>>,
) -> Result<Html<String>, StatusCode> {
    debug!("Received OAuth callback with params: {:?}", params);

    // Check if there's an error
    if let Some(error) = params.get("error") {
        error!("OAuth callback error: {}", error);
        let default_error = "Unknown error".to_string();
        let error_description = params.get("error_description").unwrap_or(&default_error);
        // Simplified error handling, avoid formatting issues
        let html = include_str!("oauth_error.html")
            .replace("{error}", error)
            .replace("{error_description}", error_description);
        return Ok(Html(html));
    }

    // Check if there's an authorization code
    if let Some(code) = params.get("code") {
        info!("Successfully received OAuth authorization code");

        // Send authorization code to main thread
        if auth_code_tx.send(code.clone()).await.is_ok() {
            return Ok(Html(
                "OAuth Authorization Successful! You can close this window.".to_string(),
            ));
        } else {
            error!("Failed to send authorization code to main thread");
            return Ok(Html(
                "OAuth Authorization Failed! Please try again.".to_string(),
            ));
        }
    }

    error!("OAuth callback missing required parameters");
    Ok(Html(
        "OAuth Authorization Failed! Missing required parameters.".to_string(),
    ))
}

/// Root path handler - provides basic information
async fn root_handler() -> Html<&'static str> {
    Html(include_str!("oauth_info.html"))
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
