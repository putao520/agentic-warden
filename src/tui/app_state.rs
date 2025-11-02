use crate::config::{AUTH_DIRECTORY, AUTH_FILE_NAME};
use crate::process_tree::get_process_name;
use crate::registry::{RegistryEntry, RegistryError, TaskRegistry};
use crate::sync::oauth_client::{OAuthConfig, OAuthTokenResponse};
use crate::sync::smart_oauth::{AuthState, SmartOAuthAuthenticator};
use crate::task_record::TaskStatus;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use thiserror::Error;

/// Shared application state container used by the TUI.
#[derive(Clone, Default)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

#[derive(Default)]
struct AppStateInner {
    oauth: RwLock<OAuthUiState>,
    providers: RwLock<ProviderUiState>,
    sync: RwLock<SyncUiState>,
    tasks: RwLock<TaskUiState>,
    authenticators: RwLock<HashMap<String, SmartOAuthAuthenticator>>,
}

impl AppState {
    /// Get a shared global instance of the application state.
    pub fn global() -> &'static AppState {
        static INSTANCE: OnceLock<AppState> = OnceLock::new();
        INSTANCE.get_or_init(AppState::new)
    }

    /// Create a new application state instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a SmartOAuth authenticator for a provider.
    pub fn register_authenticator(
        &self,
        provider: impl Into<String>,
        authenticator: SmartOAuthAuthenticator,
    ) {
        self.inner
            .authenticators
            .write()
            .insert(provider.into(), authenticator);
    }

    /// Get a previously registered authenticator.
    pub fn authenticator(&self, provider: &str) -> Option<SmartOAuthAuthenticator> {
        self.inner.authenticators.read().get(provider).cloned()
    }

    /// Ensure an authenticator exists for the given provider, loading credentials on demand.
    pub fn ensure_authenticator(
        &self,
        provider: impl Into<String>,
    ) -> Result<SmartOAuthAuthenticator, AppStateError> {
        let provider = provider.into();

        if let Some(existing) = self.authenticator(&provider) {
            return Ok(existing);
        }

        let mut store = Self::load_auth_store()?;
        store.ensure_credentials()?;

        let mut config = OAuthConfig::default();
        config.client_id = store.client_id.clone();
        config.client_secret = store.client_secret.clone();
        config.access_token = store.access_token.clone();
        config.refresh_token = store.refresh_token.clone();
        config.expires_in = store.remaining_seconds().unwrap_or(0);
        config.token_type = store
            .token_type
            .clone()
            .unwrap_or_else(|| "Bearer".to_string());
        config.scopes = Self::default_scopes();

        let authenticator = SmartOAuthAuthenticator::new(config);
        self.register_authenticator(provider.clone(), authenticator.clone());

        // Persist credentials if they were filled from environment variables.
        Self::save_auth_store(&store)?;

        Ok(authenticator)
    }

    /// Create a new OAuth flow entry for the UI to track.
    pub fn create_oauth_flow(
        &self,
        provider: impl Into<String>,
        flow_id: impl Into<String>,
        verifier: Option<String>,
        redirect_uri: Option<String>,
    ) -> String {
        let provider = provider.into();
        let flow_id = flow_id.into();
        let now = Utc::now();

        let mut flows = self.inner.oauth.write();
        flows.flows.insert(
            flow_id.clone(),
            OAuthFlow {
                id: flow_id.clone(),
                provider,
                state: AuthState::Initializing,
                verifier,
                redirect_uri,
                auth_url: None,
                created_at: now,
                updated_at: now,
            },
        );

        flow_id
    }

    /// Update the current OAuth state for a flow.
    pub fn update_oauth_state(
        &self,
        flow_id: &str,
        new_state: AuthState,
    ) -> Result<(), AppStateError> {
        let mut flows = self.inner.oauth.write();
        {
            let flow = flows
                .flows
                .get_mut(flow_id)
                .ok_or_else(|| AppStateError::UnknownOAuthFlow(flow_id.to_string()))?;

            flow.state = new_state.clone();
            flow.updated_at = Utc::now();
        }

        flows.last_state = Some(new_state.clone());
        flows.last_error = match new_state {
            AuthState::Error { ref message } => Some(message.clone()),
            _ => None,
        };

        Ok(())
    }

    /// Store the latest OAuth authorisation URL.
    pub fn set_oauth_url(
        &self,
        flow_id: &str,
        url: impl Into<String>,
    ) -> Result<(), AppStateError> {
        let mut flows = self.inner.oauth.write();
        let flow = flows
            .flows
            .get_mut(flow_id)
            .ok_or_else(|| AppStateError::UnknownOAuthFlow(flow_id.to_string()))?;

        flow.auth_url = Some(url.into());
        flow.updated_at = Utc::now();
        Ok(())
    }

    /// Persist tokens generated during an OAuth flow.
    pub fn set_auth_token(
        &self,
        provider: impl Into<String>,
        scope: impl Into<String>,
        token: impl Into<String>,
        expires_at: Option<DateTime<Utc>>,
    ) {
        let provider = provider.into();
        let scope = scope.into();
        let mut flows = self.inner.oauth.write();
        flows
            .tokens
            .entry(provider)
            .or_default()
            .insert(scope, StoredToken::new(token.into(), expires_at));
    }

    /// Persist OAuth success details to disk and update in-memory caches.
    pub fn persist_oauth_success(
        &self,
        provider: impl Into<String>,
        response: &OAuthTokenResponse,
    ) -> Result<(), AppStateError> {
        let provider = provider.into();
        let mut store = Self::load_auth_store()?;
        store.access_token = Some(response.access_token.clone());
        if let Some(refresh) = &response.refresh_token {
            store.refresh_token = Some(refresh.clone());
        }
        store.token_type = Some(response.token_type.clone());
        store.scope = response.scope.clone();

        let expires_at = Utc::now() + Duration::seconds(response.expires_in as i64);
        store.expires_at = Some(expires_at.timestamp());

        Self::save_auth_store(&store)?;

        let scope_key = response
            .scope
            .clone()
            .unwrap_or_else(|| Self::default_scopes().join(" "));

        self.set_auth_token(
            provider,
            scope_key,
            response.access_token.clone(),
            Some(expires_at),
        );
        Ok(())
    }

    /// Get a snapshot of the current OAuth UI data.
    pub fn oauth_state(&self) -> OAuthUiState {
        self.inner.oauth.read().clone()
    }

    /// Update a provider connection status entry.
    pub fn set_provider_connection_status(
        &self,
        provider: impl Into<String>,
        status: ProviderConnectionStatus,
    ) {
        let provider = provider.into();
        let mut providers = self.inner.providers.write();
        providers.connection_status.insert(provider, status);
    }

    /// Select the provider currently highlighted in the UI.
    pub fn set_selected_provider(&self, provider: Option<String>) {
        self.inner.providers.write().selected = provider;
    }

    /// Mark the provider that is being edited.
    pub fn set_editing_provider(&self, provider: Option<String>) {
        self.inner.providers.write().editing = provider;
    }

    /// Update the known provider catalog.
    pub fn set_provider_catalog(&self, catalog: ProvidersCatalog) {
        self.inner.providers.write().providers = catalog;
    }

    /// Get a snapshot of the provider view model.
    pub fn provider_state(&self) -> ProviderUiState {
        self.inner.providers.read().clone()
    }

    /// Update the sync progress for push or pull operations.
    pub fn set_sync_progress(&self, kind: TransferKind, progress: TransferProgress) {
        let mut sync = self.inner.sync.write();
        match kind {
            TransferKind::Push => sync.push = Some(progress),
            TransferKind::Pull => sync.pull = Some(progress),
        }
    }

    /// Clear sync progress information.
    pub fn clear_sync_progress(&self, kind: TransferKind) {
        let mut sync = self.inner.sync.write();
        match kind {
            TransferKind::Push => sync.push = None,
            TransferKind::Pull => sync.pull = None,
        }
    }

    /// Snapshot the current sync progress state.
    pub fn sync_state(&self) -> SyncUiState {
        self.inner.sync.read().clone()
    }

    /// Update the latest task list from the shared registry.
    pub fn refresh_tasks_from_registry(
        &self,
        registry: &TaskRegistry,
    ) -> Result<(), AppStateError> {
        let entries = registry.entries()?;
        let mut snapshots: Vec<TaskSnapshot> =
            entries.into_iter().map(TaskSnapshot::from).collect();
        snapshots.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        let mut tasks = self.inner.tasks.write();
        tasks.tasks = snapshots;
        tasks.last_refresh = Some(Utc::now());
        Ok(())
    }

    /// Manually set the task list (useful for tests).
    pub fn set_tasks(&self, tasks_list: Vec<TaskSnapshot>) {
        let mut tasks = self.inner.tasks.write();
        tasks.tasks = tasks_list;
        tasks.last_refresh = Some(Utc::now());
    }

    /// Retrieve the latest task view information.
    pub fn task_state(&self) -> TaskUiState {
        self.inner.tasks.read().clone()
    }

    /// Push a message into the global notification buffer.
    pub fn push_global_message(&self, message: impl Into<String>) {
        let mut providers = self.inner.providers.write();
        providers.messages.push_back(GlobalMessage {
            message: message.into(),
            timestamp: Utc::now(),
        });

        while providers.messages.len() > providers.max_messages {
            providers.messages.pop_front();
        }
    }

    /// Pop the oldest global message.
    pub fn pop_global_message(&self) -> Option<GlobalMessage> {
        self.inner.providers.write().messages.pop_front()
    }

    /// Create a serialisable snapshot of the entire state tree.
    pub fn create_snapshot(&self) -> AppStateSnapshot {
        AppStateSnapshot {
            oauth: self.inner.oauth.read().clone(),
            provider: self.inner.providers.read().clone(),
            sync: self.inner.sync.read().clone(),
            tasks: self.inner.tasks.read().clone(),
        }
    }

    /// Restore the complete application state from a snapshot.
    pub fn restore_from_snapshot(&self, snapshot: AppStateSnapshot) {
        *self.inner.oauth.write() = snapshot.oauth;
        *self.inner.providers.write() = snapshot.provider;
        *self.inner.sync.write() = snapshot.sync;
        *self.inner.tasks.write() = snapshot.tasks;
    }

    fn default_scopes() -> Vec<String> {
        vec![
            "https://www.googleapis.com/auth/drive.file".to_string(),
            "https://www.googleapis.com/auth/drive.metadata.readonly".to_string(),
        ]
    }

    fn auth_store_path() -> Result<PathBuf, AppStateError> {
        let home_dir = dirs::home_dir().ok_or(AppStateError::MissingHomeDirectory)?;
        Ok(home_dir.join(AUTH_DIRECTORY).join(AUTH_FILE_NAME))
    }

    fn load_auth_store() -> Result<AuthFileState, AppStateError> {
        let path = Self::auth_store_path()?;
        if !path.exists() {
            return Ok(AuthFileState::default());
        }

        let contents = fs::read_to_string(path)?;
        let store = serde_json::from_str::<AuthFileState>(&contents)?;
        Ok(store)
    }

    fn save_auth_store(store: &AuthFileState) -> Result<(), AppStateError> {
        let path = Self::auth_store_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(store)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

/// App state level error.
#[derive(Debug, Error)]
pub enum AppStateError {
    #[error("oauth flow {0} not found")]
    UnknownOAuthFlow(String),
    #[error("registry error: {0}")]
    Registry(#[from] RegistryError),
    #[error("failed to locate home directory for credentials")]
    MissingHomeDirectory,
    #[error("oauth credentials missing: {0}")]
    Credentials(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse oauth state: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AuthFileState {
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    client_secret: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    expires_at: Option<i64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

impl AuthFileState {
    fn ensure_credentials(&mut self) -> Result<(), AppStateError> {
        if self.client_id.trim().is_empty() {
            self.client_id = env::var("GOOGLE_CLIENT_ID").map_err(|_| {
                AppStateError::Credentials(
                    "Missing GOOGLE_CLIENT_ID environment variable and auth.json entry".into(),
                )
            })?;
        }

        if self.client_secret.trim().is_empty() {
            self.client_secret = env::var("GOOGLE_CLIENT_SECRET").map_err(|_| {
                AppStateError::Credentials(
                    "Missing GOOGLE_CLIENT_SECRET environment variable and auth.json entry".into(),
                )
            })?;
        }

        Ok(())
    }

    fn remaining_seconds(&self) -> Option<u64> {
        let expires_at = self.expires_at?;
        let expiry = DateTime::<Utc>::from_timestamp(expires_at, 0)?;
        let now = Utc::now();
        if expiry <= now {
            None
        } else {
            Some((expiry - now).num_seconds() as u64)
        }
    }
}

/// Stores information about a single OAuth flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlow {
    pub id: String,
    pub provider: String,
    pub state: AuthState,
    pub verifier: Option<String>,
    pub redirect_uri: Option<String>,
    pub auth_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Persisted OAuth UI data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OAuthUiState {
    pub flows: HashMap<String, OAuthFlow>,
    pub tokens: HashMap<String, HashMap<String, StoredToken>>,
    pub last_state: Option<AuthState>,
    pub last_error: Option<String>,
}

/// Stored token entry including expiry hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub token: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl StoredToken {
    pub fn new(token: String, expires_at: Option<DateTime<Utc>>) -> Self {
        Self { token, expires_at }
    }
}

/// Connection status for providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderConnectionStatus {
    Unknown,
    Authenticating,
    Connected,
    Disconnected,
    Error(String),
}

impl Default for ProviderConnectionStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Provider catalog snapshot used by the UI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersCatalog {
    pub providers: HashMap<String, ProviderSummary>,
    pub default: Option<String>,
}

/// Minimal provider summary for UI presentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub name: String,
    pub description: String,
    pub ai_types: Vec<String>,
    pub has_token: bool,
    pub official: bool,
}

/// Provider related UI data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUiState {
    pub selected: Option<String>,
    pub editing: Option<String>,
    pub connection_status: HashMap<String, ProviderConnectionStatus>,
    pub messages: VecDeque<GlobalMessage>,
    #[serde(default = "default_message_limit")]
    pub max_messages: usize,
    pub providers: ProvidersCatalog,
}

impl Default for ProviderUiState {
    fn default() -> Self {
        Self {
            selected: None,
            editing: None,
            connection_status: HashMap::new(),
            messages: VecDeque::new(),
            max_messages: default_message_limit(),
            providers: ProvidersCatalog::default(),
        }
    }
}

const fn default_message_limit() -> usize {
    64
}

/// Sync transfer type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferKind {
    Push,
    Pull,
}

/// Sync pipeline phase.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncPhase {
    Idle,
    Preparing,
    Compressing,
    Uploading,
    Downloading,
    Verifying,
    Applying,
    Completed,
    Failed,
}

impl Default for SyncPhase {
    fn default() -> Self {
        SyncPhase::Idle
    }
}

/// Progress structure shared by push and pull screens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub phase: SyncPhase,
    pub percent: u8,
    pub message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished: bool,
    pub error: Option<String>,
}

impl TransferProgress {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            phase: SyncPhase::Idle,
            percent: 0,
            message: None,
            started_at: now,
            updated_at: now,
            finished: false,
            error: None,
        }
    }

    pub fn with_phase(mut self, phase: SyncPhase) -> Self {
        self.phase = phase;
        self.updated_at = Utc::now();
        self
    }

    pub fn with_percent(mut self, percent: u8) -> Self {
        self.percent = percent.min(100);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_message(mut self, message: Option<String>) -> Self {
        self.message = message;
        self.updated_at = Utc::now();
        self
    }

    pub fn mark_finished(mut self, error: Option<String>) -> Self {
        self.finished = error.is_none();
        self.error = error;
        self.updated_at = Utc::now();
        self
    }
}

impl Default for TransferProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Sync view model tracking push and pull operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncUiState {
    pub push: Option<TransferProgress>,
    pub pull: Option<TransferProgress>,
}

/// Snapshot of a running or recently completed task used by status-aware screens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub pid: u32,
    pub log_id: String,
    pub log_path: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
    pub exit_code: Option<i32>,
    pub manager_pid: Option<u32>,
    pub manager_name: Option<String>,
    pub root_parent_pid: Option<u32>,
    pub root_parent_name: Option<String>,
    pub process_chain: Vec<u32>,
    pub process_tree_depth: usize,
    pub cleanup_reason: Option<String>,
    pub process_name: Option<String>,
}

impl From<RegistryEntry> for TaskSnapshot {
    fn from(entry: RegistryEntry) -> Self {
        let task = entry.record;
        let manager_pid = task.manager_pid;
        let manager_name = manager_pid.and_then(|pid| get_process_name(pid));
        let root_parent_pid = task.root_parent_pid;
        let root_parent_name = root_parent_pid.and_then(|pid| get_process_name(pid));
        let process_name = get_process_name(entry.pid);

        Self {
            pid: entry.pid,
            log_id: task.log_id,
            log_path: task.log_path,
            status: task.status,
            started_at: task.started_at,
            completed_at: task.completed_at,
            result: task.result,
            exit_code: task.exit_code,
            manager_pid,
            manager_name,
            root_parent_pid,
            root_parent_name,
            process_chain: task.process_chain,
            process_tree_depth: task.process_tree_depth,
            cleanup_reason: task.cleanup_reason,
            process_name,
        }
    }
}

/// Task centric UI data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskUiState {
    pub tasks: Vec<TaskSnapshot>,
    pub last_refresh: Option<DateTime<Utc>>,
}

/// Global message queue entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMessage {
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Serialisable snapshot of the complete application state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateSnapshot {
    pub oauth: OAuthUiState,
    pub provider: ProviderUiState,
    pub sync: SyncUiState,
    pub tasks: TaskUiState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn create_and_update_oauth_flow() {
        let state = AppState::new();
        let flow_id = state.create_oauth_flow("provider", "flow", None, None);
        state
            .update_oauth_state(&flow_id, AuthState::WaitingForCode { url: "url".into() })
            .unwrap();

        let oauth_state = state.oauth_state();
        assert!(oauth_state.flows.contains_key(&flow_id));
        assert!(matches!(
            oauth_state.last_state,
            Some(AuthState::WaitingForCode { .. })
        ));
    }

    #[test]
    fn provider_status_updates() {
        let state = AppState::new();
        state.set_provider_connection_status("provider", ProviderConnectionStatus::Connected);

        let provider_state = state.provider_state();
        assert!(matches!(
            provider_state
                .connection_status
                .get("provider")
                .cloned()
                .unwrap_or_default(),
            ProviderConnectionStatus::Connected
        ));
    }

    #[test]
    fn registers_and_retrieves_authenticator() {
        let state = AppState::new();
        let authenticator = SmartOAuthAuthenticator::default();
        state.register_authenticator("primary", authenticator.clone());

        assert!(state.authenticator("primary").is_some());
        assert!(state.authenticator("secondary").is_none());
    }

    #[test]
    fn stores_scoped_oauth_tokens() {
        let state = AppState::new();
        let expires = Utc::now() + Duration::hours(1);
        state.set_auth_token("provider", "scope", "token-value", Some(expires));

        let oauth_state = state.oauth_state();
        let stored = oauth_state
            .tokens
            .get("provider")
            .and_then(|scopes| scopes.get("scope"))
            .expect("token stored");
        assert_eq!(stored.token, "token-value");
        assert_eq!(stored.expires_at.unwrap().timestamp(), expires.timestamp());
    }

    #[test]
    fn sync_progress_management() {
        let state = AppState::new();
        let progress = TransferProgress::new()
            .with_phase(SyncPhase::Compressing)
            .with_percent(42)
            .with_message(Some("compressing".to_string()));

        state.set_sync_progress(TransferKind::Push, progress.clone());
        let sync_state = state.sync_state();
        assert_eq!(sync_state.push.as_ref().unwrap().percent, 42);
        assert!(matches!(
            sync_state.push.as_ref().unwrap().phase,
            SyncPhase::Compressing
        ));

        state.clear_sync_progress(TransferKind::Push);
        assert!(state.sync_state().push.is_none());
    }

    #[test]
    fn global_message_queue_respects_limit() {
        let state = AppState::new();
        {
            let mut providers = state.inner.providers.write();
            providers.max_messages = 2;
        }

        state.push_global_message("one");
        state.push_global_message("two");
        state.push_global_message("three");

        let first = state.pop_global_message().unwrap();
        let second = state.pop_global_message().unwrap();

        assert_eq!(first.message, "two");
        assert_eq!(second.message, "three");
        assert!(state.pop_global_message().is_none());
    }

    #[test]
    fn snapshot_roundtrip() {
        let state = AppState::new();
        let flow_id = state.create_oauth_flow("provider", "flow", None, None);
        state
            .update_oauth_state(
                &flow_id,
                AuthState::Authenticated {
                    access_token: Some("token".into()),
                    refresh_token: None,
                    expires_at: None,
                },
            )
            .unwrap();
        state.set_provider_connection_status("provider", ProviderConnectionStatus::Connected);

        let snapshot = state.create_snapshot();

        let restored = AppState::new();
        restored.restore_from_snapshot(snapshot);

        let provider_state = restored.provider_state();
        assert!(matches!(
            provider_state
                .connection_status
                .get("provider")
                .cloned()
                .unwrap_or_default(),
            ProviderConnectionStatus::Connected
        ));
    }
}
