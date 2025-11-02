//! Terminal User Interface (TUI) module
//!
//! Provides unified TUI screens for all interactive features

pub mod app;
pub mod app_state;
pub mod event;
pub mod screens;
pub mod widgets;

pub use app::TuiApp;
pub use app_state::{
    AppState, AppStateSnapshot, GlobalMessage, OAuthFlow, OAuthUiState, ProviderConnectionStatus,
    ProviderSummary, ProviderUiState, ProvidersCatalog, SyncPhase, SyncUiState, TaskSnapshot,
    TaskUiState, TransferKind, TransferProgress,
};
pub use event::{Event, EventHandler};
