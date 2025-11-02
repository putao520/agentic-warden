use agentic_warden::sync::smart_oauth::AuthState;
use agentic_warden::tui::app_state::{
    AppState, ProviderConnectionStatus, ProviderSummary, ProvidersCatalog, SyncPhase, TransferKind,
    TransferProgress,
};
use chrono::Utc;

#[test]
fn end_to_end_app_state_snapshot_flow() {
    let state = AppState::new();

    // Register authenticator to ensure provider is tracked.
    let authenticator = agentic_warden::sync::smart_oauth::SmartOAuthAuthenticator::default();
    state.register_authenticator("primary", authenticator);

    // Create OAuth flow and update metadata.
    let flow_id = state.create_oauth_flow("primary", "flow-1", None, None);
    state
        .set_oauth_url(&flow_id, "https://example.com/auth")
        .unwrap();
    state
        .update_oauth_state(
            &flow_id,
            AuthState::WaitingForCode {
                url: "https://example.com/auth".into(),
            },
        )
        .unwrap();
    state.set_auth_token("primary", "drive.scope", "token-value", Some(Utc::now()));

    // Configure provider view state.
    let mut catalog = ProvidersCatalog::default();
    catalog.default = Some("primary".into());
    catalog.providers.insert(
        "primary".into(),
        ProviderSummary {
            name: "primary".into(),
            description: "Primary provider".into(),
            ai_types: vec!["codex".into()],
            has_token: true,
            official: true,
        },
    );
    state.set_provider_catalog(catalog);
    state.set_provider_connection_status("primary", ProviderConnectionStatus::Connected);
    state.set_selected_provider(Some("primary".into()));

    // Record sync progress.
    let progress = TransferProgress::new()
        .with_phase(SyncPhase::Preparing)
        .with_percent(10)
        .with_message(Some("starting sync".into()));
    state.set_sync_progress(TransferKind::Push, progress);

    // Add global messages.
    state.push_global_message("Sync started");

    // Take snapshot and restore into a new state.
    let snapshot = state.create_snapshot();
    let restored = AppState::new();
    restored.restore_from_snapshot(snapshot);

    let oauth = restored.oauth_state();
    assert!(oauth.flows.contains_key("flow-1"));
    assert!(matches!(
        oauth.last_state,
        Some(AuthState::WaitingForCode { .. })
    ));
    let stored_token = oauth
        .tokens
        .get("primary")
        .and_then(|scopes| scopes.get("drive.scope"))
        .expect("token present");
    assert_eq!(stored_token.token, "token-value");

    let provider = restored.provider_state();
    assert_eq!(provider.selected.as_deref(), Some("primary"));
    assert!(matches!(
        provider
            .connection_status
            .get("primary")
            .cloned()
            .unwrap_or_default(),
        ProviderConnectionStatus::Connected
    ));
    assert!(provider.providers.providers.contains_key("primary"));

    let sync_state = restored.sync_state();
    let push = sync_state.push.expect("push progress");
    assert_eq!(push.percent, 10);
    assert!(matches!(push.phase, SyncPhase::Preparing));

    let message = restored.pop_global_message().expect("message present");
    assert_eq!(message.message, "Sync started");
    assert!(restored.pop_global_message().is_none());
}
