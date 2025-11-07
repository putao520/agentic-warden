use crate::sync::error::SyncResult;
use crate::sync::oauth_client::{OAuthClient, OAuthConfig, OAuthTokenResponse};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use console::Term;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Represents the high level authentication state exposed to the TUI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthState {
    /// Initialising the authentication flow and validating configuration.
    Initializing,
    /// Waiting for the user to authorise in the browser and supply the code.
    WaitingForCode { url: String },
    /// Authentication succeeded and we have usable tokens.
    Authenticated {
        access_token: Option<String>,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    },
    /// Authentication failed – store a short message so the UI can surface it.
    Error { message: String },
}

impl AuthState {
    fn with_error<E: std::fmt::Display>(err: E) -> Self {
        Self::Error {
            message: err.to_string(),
        }
    }
}

struct SmartOAuthInner {
    client: Mutex<OAuthClient>,
    state: RwLock<AuthState>,
    last_url: RwLock<Option<String>>,
}

/// Thin wrapper around `OAuthClient` that tracks high-level state for the TUI layer.
#[derive(Clone)]
pub struct SmartOAuthAuthenticator {
    inner: Arc<SmartOAuthInner>,
}

impl SmartOAuthAuthenticator {
    /// Create a new authenticator using an `OAuthConfig`.
    /// Any pre-existing tokens inside the configuration will mark the state as authenticated.
    pub fn new(config: OAuthConfig) -> Self {
        let mut client = OAuthClient::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.refresh_token.clone(),
        );
        if !config.scopes.is_empty() {
            client = client.with_scopes(config.scopes.clone());
        }

        let initial_state = if config.access_token.is_some() || config.refresh_token.is_some() {
            AuthState::Authenticated {
                access_token: config.access_token,
                refresh_token: config.refresh_token,
                expires_at: expires_at_from_hint(config.expires_in),
            }
        } else {
            AuthState::Initializing
        };

        Self {
            inner: Arc::new(SmartOAuthInner {
                client: Mutex::new(client),
                state: RwLock::new(initial_state),
                last_url: RwLock::new(None),
            }),
        }
    }

    /// Generate an authorisation URL for the TUI and transition into `WaitingForCode`.
    pub async fn generate_auth_url_for_tui(&self) -> Result<String> {
        let url_result: Result<String> = {
            let client = self.inner.client.lock().await;
            if let Err(err) = client.validate_config() {
                Err(err)
            } else {
                client.generate_auth_url()
            }
        };

        match url_result {
            Ok(url) => {
                {
                    let mut state = self.inner.state.write().await;
                    *state = AuthState::WaitingForCode { url: url.clone() };
                }

                {
                    let mut last_url = self.inner.last_url.write().await;
                    *last_url = Some(url.clone());
                }

                Ok(url)
            }
            Err(err) => {
                {
                    let mut state = self.inner.state.write().await;
                    *state = AuthState::with_error(&err);
                }
                Err(err)
            }
        }
    }

    /// Run a full OOB authentication loop by prompting for the auth code.
    pub async fn authenticate(&self) -> Result<OAuthTokenResponse> {
        let url = self.generate_auth_url_for_tui().await?;

        println!();
        println!("Open the following URL in your browser to authorise Agentic Warden:");
        println!("{}", url);
        println!();

        print!("Enter the authorisation code provided by Google: ");
        io::stdout().flush()?;

        let term = Term::stdout();
        let code = term.read_line()?;
        self.set_auth_code(code).await
    }

    /// Provide the user supplied authorisation code.
    /// Returns an error if the authenticator is not currently waiting for a code.
    pub async fn set_auth_code(&self, code: String) -> Result<OAuthTokenResponse> {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("authorisation code cannot be empty"));
        }

        {
            let current = self.inner.state.read().await.clone();
            if !matches!(current, AuthState::WaitingForCode { .. }) {
                return Err(anyhow!(
                    "authenticator is not ready to accept an authorisation code"
                ));
            }
        }

        let exchange_result = self.exchange_code_for_tokens(trimmed.to_string()).await;
        match exchange_result {
            Ok(tokens) => {
                let mut state = self.inner.state.write().await;
                *state = AuthState::Authenticated {
                    access_token: Some(tokens.access_token.clone()),
                    refresh_token: tokens.refresh_token.clone(),
                    expires_at: expires_at_from_hint(tokens.expires_in),
                };
                Ok(tokens)
            }
            Err(err) => {
                {
                    let mut state = self.inner.state.write().await;
                    *state = AuthState::with_error(&err);
                }
                Err(err)
            }
        }
    }

    /// Get the current state snapshot.
    pub async fn get_state(&self) -> AuthState {
        self.inner.state.read().await.clone()
    }

    /// Fetch the last generated authorisation URL if available.
    pub async fn last_auth_url(&self) -> Option<String> {
        self.inner.last_url.read().await.clone()
    }

    async fn exchange_code_for_tokens(&self, code: String) -> Result<OAuthTokenResponse> {
        let mut client = self.inner.client.lock().await;
        client.exchange_code_for_tokens(&code).await
    }
}

/// Returns the default authenticator state.
impl Default for SmartOAuthAuthenticator {
    fn default() -> Self {
        Self::new(OAuthConfig::default())
    }
}

fn expires_at_from_hint(expires_in: u64) -> Option<DateTime<Utc>> {
    if expires_in == 0 {
        None
    } else {
        Some(Utc::now() + Duration::seconds(expires_in as i64))
    }
}

/// Convenience helper that integrates with the existing sync error type.
pub async fn authenticate_with_code(
    authenticator: &SmartOAuthAuthenticator,
    code: String,
) -> SyncResult<()> {
    authenticator
        .set_auth_code(code)
        .await
        .map(|_| ())
        .map_err(|e| crate::sync::error::SyncError::general(e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::oauth_client::OAuthConfig;

    #[tokio::test]
    async fn new_authenticator_starts_in_initializing() {
        let config = OAuthConfig::default();
        let auth = SmartOAuthAuthenticator::new(config);
        assert!(matches!(auth.get_state().await, AuthState::Initializing));
    }

    #[tokio::test]
    async fn authenticator_with_tokens_is_authenticated() {
        let config = OAuthConfig {
            client_id: "id".into(),
            client_secret: "secret".into(),
            access_token: Some("token".into()),
            refresh_token: None,
            ..OAuthConfig::default()
        };
        let auth = SmartOAuthAuthenticator::new(config);
        match auth.get_state().await {
            AuthState::Authenticated { access_token, .. } => {
                assert_eq!(access_token, Some("token".to_string()));
            }
            other => panic!("unexpected state: {:?}", other),
        }
    }

    #[tokio::test]
    async fn url_generation_succeeds_and_updates_state() {
        let config = OAuthConfig {
            client_id: "id".into(),
            client_secret: "secret".into(),
            ..OAuthConfig::default()
        };
        let auth = SmartOAuthAuthenticator::new(config);
        let url = auth.generate_auth_url_for_tui().await.unwrap();
        assert!(url.contains("https://accounts.google.com/o/oauth2/v2/auth"));
        assert!(matches!(
            auth.get_state().await,
            AuthState::WaitingForCode { .. }
        ));
        assert_eq!(auth.last_auth_url().await, Some(url));
    }

    #[tokio::test]
    async fn invalid_config_sets_error_state() {
        let config = OAuthConfig {
            client_id: "".into(),
            client_secret: "".into(),
            ..OAuthConfig::default()
        };
        let auth = SmartOAuthAuthenticator::new(config);
        let err = auth.generate_auth_url_for_tui().await.unwrap_err();
        assert!(err.to_string().contains("Client ID"));
        assert!(matches!(auth.get_state().await, AuthState::Error { .. }));
    }

    #[tokio::test]
    async fn set_auth_code_requires_waiting_state() {
        let config = OAuthConfig {
            client_id: "id".into(),
            client_secret: "secret".into(),
            ..OAuthConfig::default()
        };
        let auth = SmartOAuthAuthenticator::new(config);
        let err = auth.set_auth_code("code".into()).await.unwrap_err();
        assert!(err
            .to_string()
            .contains("authenticator is not ready to accept"));
    }

    #[tokio::test]
    async fn set_auth_code_rejects_empty_input() {
        let config = OAuthConfig {
            client_id: "id".into(),
            client_secret: "secret".into(),
            ..OAuthConfig::default()
        };
        let auth = SmartOAuthAuthenticator::new(config);
        // Transition into WaitingForCode using the real client; this should not hit the network.
        auth.generate_auth_url_for_tui().await.unwrap();
        let err = auth.set_auth_code("   ".into()).await.unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
    }
}
