#![allow(dead_code)] // TUI状态管理，部分枚举变体当前未使用

//! Shared TUI application state with global access.
//!
//! Exposes a lightweight singleton that coordinates OAuth flows and sync
//! progress so that multiple screens can exchange information without
//! plumbing large amounts of state through constructors.

use crate::{
    config::{AUTH_DIRECTORY, AUTH_FILE_NAME},
    provider::config::Provider as ProviderConfig,
    storage::RegistryEntry,
    sync::smart_oauth::{AuthState, SmartOAuthAuthenticator},
    task_record::{TaskRecord, TaskStatus},
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, TimeZone, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration as StdDuration, Instant},
};

static GLOBAL_APP_STATE: OnceLock<AppState> = OnceLock::new();

/// In-memory representation of the shared application state.
pub struct AppState {
    providers: RwLock<Vec<ProviderBinding>>,
    default_provider: RwLock<Option<String>>,
    tasks: RwLock<HashMap<u32, TaskSnapshot>>,
    sync_progress: RwLock<HashMap<TransferKind, TransferProgress>>,
    authenticators: RwLock<HashMap<String, SmartOAuthAuthenticator>>,
    oauth_flows: RwLock<HashMap<String, OAuthFlow>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
            default_provider: RwLock::new(None),
            tasks: RwLock::new(HashMap::new()),
            sync_progress: RwLock::new(HashMap::new()),
            authenticators: RwLock::new(HashMap::new()),
            oauth_flows: RwLock::new(HashMap::new()),
        }
    }
}

impl AppState {
    /// Return a reference to the global state, initialising it on first use.
    pub fn global() -> &'static Self {
        GLOBAL_APP_STATE.get_or_init(Self::default)
    }

    /// Replace provider snapshot (typically loaded from disk).
    pub fn set_providers<I>(&self, providers: I, default_provider: Option<String>)
    where
        I: IntoIterator<Item = (String, ProviderConfig)>,
    {
        let default = default_provider.clone();
        let mut list: Vec<ProviderBinding> = providers
            .into_iter()
            .map(|(id, provider)| {
                let is_default = default.as_ref().map(|d| d == &id).unwrap_or(false);
                ProviderBinding {
                    id,
                    provider,
                    is_default,
                }
            })
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));

        *self.default_provider.write() = default;
        *self.providers.write() = list;
    }

    /// Return providers currently cached in the shared state.
    pub fn providers(&self) -> Vec<ProviderBinding> {
        self.providers.read().clone()
    }

    /// Return the configured default provider, if any.
    pub fn default_provider(&self) -> Option<String> {
        self.default_provider.read().clone()
    }

    /// Replace all cached tasks with the provided snapshots.
    pub fn replace_tasks(&self, snapshots: Vec<TaskSnapshot>) {
        let mut tasks = self.tasks.write();
        tasks.clear();
        for snapshot in snapshots {
            tasks.insert(snapshot.pid, snapshot);
        }
    }

    /// Replace cached tasks using registry entries.
    pub fn replace_tasks_from_registry(&self, entries: Vec<RegistryEntry>) {
        let snapshots = entries
            .into_iter()
            .map(TaskSnapshot::from_registry_entry)
            .collect();
        self.replace_tasks(snapshots);
    }

    /// Get a cloned snapshot of all tasks.
    pub fn tasks_snapshot(&self) -> Vec<TaskSnapshot> {
        self.tasks.read().values().cloned().collect()
    }

    /// Store transfer progress for the specified transfer kind.
    pub fn set_sync_progress(&self, kind: TransferKind, mut progress: TransferProgress) {
        progress.kind = kind;
        self.sync_progress.write().insert(kind, progress);
    }

    /// Clear previously stored transfer progress.
    pub fn clear_sync_progress(&self, kind: TransferKind) {
        self.sync_progress.write().remove(&kind);
    }

    /// Fetch the latest transfer progress for the provided kind.
    pub fn get_sync_progress(&self, kind: &TransferKind) -> Option<TransferProgress> {
        self.sync_progress.read().get(kind).cloned()
    }

    /// Return a snapshot of the cached Google Drive credential state.
    pub fn google_drive_auth_snapshot(&self) -> GoogleDriveAuthSnapshot {
        match self.load_oauth_state() {
            Ok(Some(state)) => {
                let expires_at = state
                    .expires_at
                    .and_then(|ts| Utc.timestamp_opt(ts, 0).single());
                GoogleDriveAuthSnapshot {
                    configured: true,
                    has_refresh_token: state
                        .refresh_token
                        .as_ref()
                        .map(|token| !token.trim().is_empty())
                        .unwrap_or(false),
                    expires_at,
                    error: None,
                }
            }
            Ok(None) => GoogleDriveAuthSnapshot {
                configured: false,
                has_refresh_token: false,
                expires_at: None,
                error: None,
            },
            Err(err) => GoogleDriveAuthSnapshot {
                configured: false,
                has_refresh_token: false,
                expires_at: None,
                error: Some(err.to_string()),
            },
        }
    }

    /// Fetch OAuth flows that have been updated recently so other screens can
    /// surface their status. Older flows are pruned to bound memory usage.
    pub fn recent_oauth_flows(&self, max_age: StdDuration) -> Vec<OAuthFlow> {
        let mut flows = self.oauth_flows.write();
        let now = Instant::now();
        flows.retain(|_, flow| {
            now.checked_duration_since(flow.updated_at)
                .map(|age| age <= max_age)
                .unwrap_or(true)
        });

        let mut snapshot: Vec<_> = flows.values().cloned().collect();
        snapshot.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        snapshot
    }

    /// Ensure a [`SmartOAuthAuthenticator`] exists for the requested provider.
    /// STUB: OAuth TUI flow not fully implemented for simplified sync module
    pub fn ensure_authenticator(&self, provider: &str) -> Result<SmartOAuthAuthenticator> {
        Err(anyhow!("OAuth TUI flow not implemented for {}", provider))
    }

    /// Persist new OAuth tokens returned by the authenticator and refresh the cache.
    /// STUB: OAuth TUI flow not fully implemented for simplified sync module
    pub fn persist_oauth_success(
        &self,
        provider: &str,
        _response: &serde_json::Value,
    ) -> Result<()> {
        Err(anyhow!(
            "OAuth TUI persistence not implemented for {}",
            provider
        ))
    }

    /// Create a new OAuth flow entry so other screens can query its state.
    pub fn create_oauth_flow(
        &self,
        provider: &str,
        flow_id: String,
        auth_url: Option<String>,
        state: Option<AuthState>,
    ) {
        let auth_state = state.unwrap_or(AuthState::Initializing);
        let mut dialog = AuthDialog::new();
        if let Some(url) = auth_url {
            dialog.auth_url = url;
        }
        dialog.status = AuthStatus::from_auth_state(&auth_state);

        self.oauth_flows.write().insert(
            flow_id,
            OAuthFlow {
                provider: provider.to_string(),
                dialog,
                auth_state,
                started_at: Instant::now(),
                updated_at: Instant::now(),
            },
        );
    }

    /// Update the stored authorisation URL for a flow.
    pub fn set_oauth_url(&self, flow_id: &str, url: &str) -> Result<()> {
        let mut flows = self.oauth_flows.write();
        match flows.get_mut(flow_id) {
            Some(flow) => {
                flow.dialog.auth_url = url.to_string();
                flow.updated_at = Instant::now();
            }
            None => {
                let mut dialog = AuthDialog::new();
                dialog.auth_url = url.to_string();
                flows.insert(
                    flow_id.to_string(),
                    OAuthFlow {
                        provider: String::new(),
                        dialog,
                        auth_state: AuthState::Initializing,
                        started_at: Instant::now(),
                        updated_at: Instant::now(),
                    },
                );
            }
        }
        Ok(())
    }

    /// Update the high level OAuth state for display.
    pub fn update_oauth_state(&self, flow_id: &str, state: AuthState) -> Result<()> {
        let mut flows = self.oauth_flows.write();
        let flow = flows
            .get_mut(flow_id)
            .ok_or_else(|| anyhow!("OAuth flow '{flow_id}' not found"))?;

        flow.auth_state = state.clone();
        flow.dialog.status = AuthStatus::from_auth_state(&state);
        flow.updated_at = Instant::now();
        Ok(())
    }

    /// Retrieve a stored OAuth flow snapshot if present.
    pub fn get_oauth_flow(&self, flow_id: &str) -> Option<OAuthFlow> {
        self.oauth_flows.read().get(flow_id).cloned()
    }

    fn load_oauth_config(&self, _provider: &str) -> Result<serde_json::Value> {
        // STUB: Simplified OAuth config for TUI
        Err(anyhow!(
            "OAuth config loading not implemented in simplified sync module"
        ))
    }

    fn load_oauth_state(&self) -> Result<Option<StoredOAuthState>> {
        let path = Self::auth_file_path()?;
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).with_context(|| {
            format!(
                "failed to read OAuth credentials from {}",
                path.to_string_lossy()
            )
        })?;

        let state: StoredOAuthState = serde_json::from_str(&content).with_context(|| {
            format!(
                "failed to parse OAuth credentials file {}",
                path.to_string_lossy()
            )
        })?;

        Ok(Some(state))
    }

    fn save_oauth_state(&self, state: &StoredOAuthState) -> Result<()> {
        let path = Self::auth_file_path()?;
        let content =
            serde_json::to_string_pretty(state).context("failed to serialise OAuth credentials")?;
        fs::write(&path, content).with_context(|| {
            format!(
                "failed to write OAuth credentials to {}",
                path.to_string_lossy()
            )
        })
    }

    fn auth_file_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().context("failed to determine home directory for OAuth storage")?;
        let dir = home.join(AUTH_DIRECTORY);
        fs::create_dir_all(&dir).with_context(|| {
            format!(
                "failed to create OAuth storage directory {}",
                dir.to_string_lossy()
            )
        })?;
        Ok(dir.join(AUTH_FILE_NAME))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredOAuthState {
    client_id: String,
    client_secret: String,
    refresh_token: Option<String>,
    access_token: Option<String>,
    expires_at: Option<i64>,
    token_type: Option<String>,
    scope: Option<String>,
}

/// Synchronisation phases shown in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPhase {
    Idle,
    Preparing,
    Authentication,
    Listing,
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

/// Transfer direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TransferKind {
    #[default]
    Push,
    Pull,
    Sync,
}

/// Aggregated progress information for a transfer operation.
#[derive(Debug, Clone, Default)]
pub struct TransferProgress {
    pub kind: TransferKind,
    pub phase: SyncPhase,
    pub percent: u8,
    pub current: usize,
    pub total: usize,
    pub current_file: Option<String>,
    pub message: Option<String>,
    pub speed: Option<f64>, // bytes per second
    pub eta: Option<std::time::Duration>,
}

impl TransferProgress {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_kind(kind: TransferKind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    pub fn with_phase(mut self, phase: SyncPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_percent(mut self, percent: u8) -> Self {
        self.percent = percent.min(100);
        self
    }

    pub fn with_message(mut self, message: Option<String>) -> Self {
        self.message = message;
        self
    }

    pub fn with_file(mut self, file: Option<String>) -> Self {
        self.current_file = file;
        self
    }

    pub fn progress_percent(&self) -> f32 {
        self.percent as f32
    }
}

/// Provider snapshot entry exposed to screens.
#[derive(Debug, Clone)]
pub struct ProviderBinding {
    pub id: String,
    pub provider: ProviderConfig,
    pub is_default: bool,
}

/// Task snapshot for dashboard/status screens.
#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub pid: u32,
    pub record: TaskRecord,
    pub status: TaskUiState,
    pub progress: f32,
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

impl TaskSnapshot {
    pub fn from_registry_entry(entry: RegistryEntry) -> Self {
        let RegistryEntry { pid, record, .. } = entry;
        let status = match record.status {
            TaskStatus::Running => TaskUiState::Running,
            TaskStatus::CompletedButUnread => {
                let exit_code = record.exit_code.unwrap_or(0);
                if exit_code != 0 {
                    TaskUiState::Failed(
                        record
                            .cleanup_reason
                            .clone()
                            .unwrap_or_else(|| format!("Exit code {}", exit_code)),
                    )
                } else {
                    TaskUiState::Completed
                }
            }
        };

        Self {
            pid,
            record,
            progress: if matches!(status, TaskUiState::Completed) {
                1.0
            } else {
                0.0
            },
            cpu_usage: 0.0,
            memory_usage: 0,
            status,
        }
    }
}

/// UI-facing task status.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskUiState {
    Running,
    Completed,
    Failed(String),
    Pending,
    Paused,
}

/// Dialog state surfaced to the UI when guiding the user through OAuth.
#[derive(Debug, Clone)]
pub struct AuthDialog {
    pub auth_url: String,
    pub status: AuthStatus,
}

impl AuthDialog {
    pub fn new() -> Self {
        Self {
            auth_url: String::new(),
            status: AuthStatus::Waiting,
        }
    }
}

/// High level OAuth status used by progress screens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    Waiting,
    CallbackStarted,
    Authorized,
    Failed(String),
}

impl AuthStatus {
    fn from_auth_state(state: &AuthState) -> Self {
        match state {
            AuthState::Initializing => AuthStatus::Waiting,
            AuthState::WaitingForDeviceAuth { .. } => AuthStatus::Waiting,
            AuthState::Authenticated { .. } => AuthStatus::Authorized,
            AuthState::Error { message } => AuthStatus::Failed(message.clone()),
        }
    }
}

/// Details tracked for an ongoing OAuth flow.
#[derive(Debug, Clone)]
pub struct OAuthFlow {
    pub provider: String,
    pub dialog: AuthDialog,
    pub auth_state: AuthState,
    pub started_at: Instant,
    pub updated_at: Instant,
}

/// Lightweight status summary of the cached Google Drive credentials.
#[derive(Debug, Clone, Default)]
pub struct GoogleDriveAuthSnapshot {
    pub configured: bool,
    pub has_refresh_token: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}
