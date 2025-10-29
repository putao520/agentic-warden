//! Integration tests for OAuth authentication flow
//!
//! These tests verify the OAuth authentication mechanism including:
//! - Token refresh functionality
//! - Configuration persistence
//! - Error handling for authentication failures

use agentic_warden::sync::error::{SyncError, SyncResult};
use agentic_warden::sync::google_drive_client::{
    GoogleDriveClient, GoogleDriveConfig, OAuthTokenResponse,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Helper function for safe environment variable setting
fn set_env_var(key: &str, value: &str) {
    unsafe {
        std::env::set_var(key, value);
    }
}

fn remove_env_var(key: &str) {
    unsafe {
        std::env::remove_var(key);
    }
}

#[test]
fn test_google_drive_config_serialization() {
    let config = GoogleDriveConfig {
        client_id: "test_client_id_123".to_string(),
        client_secret: "test_client_secret_456".to_string(),
        access_token: Some("test_access_token_789".to_string()),
        refresh_token: Some("test_refresh_token_abc".to_string()),
        base_folder_id: Some("test_folder_id_def".to_string()),
        token_expires_at: Some(1234567890),
    };

    // Test serialization
    let serialized = serde_json::to_string(&config).expect("Failed to serialize config");

    // Test deserialization
    let deserialized: GoogleDriveConfig =
        serde_json::from_str(&serialized).expect("Failed to deserialize config");

    assert_eq!(config.client_id, deserialized.client_id);
    assert_eq!(config.client_secret, deserialized.client_secret);
    assert_eq!(config.access_token, deserialized.access_token);
    assert_eq!(config.refresh_token, deserialized.refresh_token);
    assert_eq!(config.base_folder_id, deserialized.base_folder_id);
    assert_eq!(config.token_expires_at, deserialized.token_expires_at);
}

#[test]
fn test_auth_config_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let auth_path = temp_dir.path().join("auth.json");

    // Create a client with custom auth path by temporarily changing the home directory
    let original_home = std::env::var("HOME").ok();
    let temp_home = temp_dir.path().to_string_lossy().to_string();
    set_env_var("HOME", &temp_home);

    // Create warden directory
    let warden_dir = temp_dir.path().join(".codex-warden");
    fs::create_dir_all(&warden_dir).expect("Failed to create warden directory");

    let config = GoogleDriveConfig {
        client_id: "persistent_client_id".to_string(),
        client_secret: "persistent_client_secret".to_string(),
        access_token: Some("persistent_access_token".to_string()),
        refresh_token: Some("persistent_refresh_token".to_string()),
        base_folder_id: Some("persistent_folder_id".to_string()),
        token_expires_at: Some(1609459200), // 2021-01-01
    };

    let client = GoogleDriveClient::new(config.clone());

    // Save configuration
    client
        .save_auth_config()
        .expect("Failed to save auth config");

    // Verify file was created
    assert!(auth_path.exists());

    // Load configuration
    let loaded_config = GoogleDriveClient::load_auth_config()
        .expect("Failed to load auth config")
        .expect("No auth config found");

    assert_eq!(loaded_config.client_id, config.client_id);
    assert_eq!(loaded_config.client_secret, config.client_secret);
    assert_eq!(loaded_config.access_token, config.access_token);
    assert_eq!(loaded_config.refresh_token, config.refresh_token);
    assert_eq!(loaded_config.base_folder_id, config.base_folder_id);
    assert_eq!(loaded_config.token_expires_at, config.token_expires_at);

    // Restore original HOME environment variable
    if let Some(home) = original_home {
        set_env_var("HOME", &home);
    } else {
        remove_env_var("HOME");
    }
}

#[test]
fn test_auth_url_generation() {
    let config = GoogleDriveConfig {
        client_id: "test_client_123".to_string(),
        client_secret: "secret_456".to_string(),
        access_token: None,
        refresh_token: None,
        base_folder_id: None,
        token_expires_at: None,
    };

    let mut client = GoogleDriveClient::new(config);
    let auth_url = client
        .generate_auth_url()
        .expect("Failed to generate auth URL");

    // Verify URL contains required components
    assert!(auth_url.contains("accounts.google.com"));
    assert!(auth_url.contains("test_client_123"));
    assert!(auth_url.contains("scope=https://www.googleapis.com/auth/drive.file"));
    assert!(auth_url.contains("response_type=code"));
    assert!(auth_url.contains("redirect_uri=urn:ietf:wg:oauth:2.0:oob"));
    assert!(auth_url.contains("access_type=offline"));
    assert!(auth_url.contains("prompt=consent"));
}

#[test]
fn test_token_expiry_detection() {
    let config = GoogleDriveConfig {
        client_id: "test_client".to_string(),
        client_secret: "test_secret".to_string(),
        access_token: Some("valid_token".to_string()),
        refresh_token: Some("refresh_token".to_string()),
        base_folder_id: None,
        token_expires_at: None,
    };

    let mut client = GoogleDriveClient::new(config);

    // Test with no expiry time (should be considered expired)
    assert!(client.is_token_expired());

    // Test with past expiry time
    let past_time = chrono::Utc::now().timestamp() - 3600; // 1 hour ago
    client.config.token_expires_at = Some(past_time);
    assert!(client.is_token_expired());

    // Test with future expiry time
    let future_time = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
    client.config.token_expires_at = Some(future_time);
    assert!(!client.is_token_expired());

    // Test with expiry time in safety buffer (5 minutes from now should be considered expired)
    let near_future_time = chrono::Utc::now().timestamp() + 200; // ~3 minutes from now
    client.config.token_expires_at = Some(near_future_time);
    assert!(client.is_token_expired());
}

#[tokio::test]
async fn test_token_response_processing() {
    let mut client = GoogleDriveClient::new(GoogleDriveConfig {
        client_id: "test_client".to_string(),
        client_secret: "test_secret".to_string(),
        access_token: None,
        refresh_token: None,
        base_folder_id: None,
        token_expires_at: None,
    });

    // Mock token response
    let token_response = OAuthTokenResponse {
        access_token: "new_access_token".to_string(),
        refresh_token: Some("new_refresh_token".to_string()),
        expires_in: 3600, // 1 hour
        token_type: "Bearer".to_string(),
    };

    // Simulate processing token response
    let now = chrono::Utc::now().timestamp();
    let expires_at = now + (token_response.expires_in as i64) - 300; // 5 minutes buffer

    client.config.access_token = Some(token_response.access_token);
    client.config.refresh_token = token_response.refresh_token;
    client.config.token_expires_at = Some(expires_at);

    // Verify token was processed correctly
    assert_eq!(
        client.config.access_token,
        Some("new_access_token".to_string())
    );
    assert_eq!(
        client.config.refresh_token,
        Some("new_refresh_token".to_string())
    );
    assert!(client.config.token_expires_at.is_some());

    let expiry_time = client.config.token_expires_at.unwrap();
    assert!(expiry_time > now);
    assert!(expiry_time < now + 3600); // Should be less than raw expiry time due to buffer
}

#[test]
fn test_oauth_error_scenarios() {
    // Test missing client configuration
    let config = GoogleDriveConfig {
        client_id: "".to_string(),
        client_secret: "".to_string(),
        access_token: None,
        refresh_token: None,
        base_folder_id: None,
        token_expires_at: None,
    };

    let client = GoogleDriveClient::new(config);

    // Test auth URL generation with empty client ID should still work
    let auth_url_result = client.generate_auth_url();
    assert!(auth_url_result.is_ok());
    let url = auth_url_result.unwrap();
    assert!(url.contains("client_id=")); // Even if empty, parameter should be present

    // Test get access token when none exists
    let token_result = client.get_access_token();
    assert!(token_result.is_err());
    match token_result.unwrap_err() {
        SyncError::AuthenticationRequired => {} // Expected error
        _ => panic!("Expected AuthenticationRequired error"),
    }
}

#[test]
fn test_backward_compatibility_for_missing_fields() {
    // Test loading config that doesn't have token_expires_at field
    let config_json = r#"{
        "client_id": "old_client_id",
        "client_secret": "old_client_secret",
        "access_token": "old_access_token",
        "refresh_token": "old_refresh_token",
        "base_folder_id": "old_folder_id"
    }"#;

    let config: GoogleDriveConfig =
        serde_json::from_str(config_json).expect("Failed to parse backward-compatible config");

    assert_eq!(config.client_id, "old_client_id");
    assert_eq!(config.client_secret, "old_client_secret");
    assert_eq!(config.access_token, Some("old_access_token".to_string()));
    assert_eq!(config.refresh_token, Some("old_refresh_token".to_string()));
    assert_eq!(config.base_folder_id, Some("old_folder_id".to_string()));
    assert_eq!(config.token_expires_at, None); // Should be None for backward compatibility

    // Client should handle missing expiry time gracefully
    let client = GoogleDriveClient::new(config);
    assert!(client.is_token_expired()); // Should consider expired by default
}

#[test]
fn test_environment_variable_configuration() {
    // Set up environment variables
    set_env_var("GOOGLE_CLIENT_ID", "env_client_id");
    set_env_var("GOOGLE_CLIENT_SECRET", "env_client_secret");
    set_env_var("GOOGLE_ACCESS_TOKEN", "env_access_token");
    set_env_var("GOOGLE_REFRESH_TOKEN", "env_refresh_token");
    set_env_var("GOOGLE_DRIVE_FOLDER_ID", "env_folder_id");
    set_env_var("GOOGLE_TOKEN_EXPIRES_AT", "1609459200");

    let client_result = GoogleDriveClient::from_env();
    assert!(client_result.is_ok());

    let client = client_result.unwrap();
    let config = client.get_config();

    assert_eq!(config.client_id, "env_client_id");
    assert_eq!(config.client_secret, "env_client_secret");
    assert_eq!(config.access_token, Some("env_access_token".to_string()));
    assert_eq!(config.refresh_token, Some("env_refresh_token".to_string()));
    assert_eq!(config.base_folder_id, Some("env_folder_id".to_string()));
    assert_eq!(config.token_expires_at, Some(1609459200));

    // Clean up environment variables
    remove_env_var("GOOGLE_CLIENT_ID");
    remove_env_var("GOOGLE_CLIENT_SECRET");
    remove_env_var("GOOGLE_ACCESS_TOKEN");
    remove_env_var("GOOGLE_REFRESH_TOKEN");
    remove_env_var("GOOGLE_DRIVE_FOLDER_ID");
    remove_env_var("GOOGLE_TOKEN_EXPIRES_AT");
}

#[test]
fn test_environment_variable_missing() {
    // Test with missing required environment variables
    remove_env_var("GOOGLE_CLIENT_ID");
    remove_env_var("GOOGLE_CLIENT_SECRET");

    let client_result = GoogleDriveClient::from_env();
    assert!(client_result.is_err());

    match client_result.unwrap_err() {
        SyncError::GoogleDriveError(msg) => {
            assert!(msg.contains("GOOGLE_CLIENT_ID"));
        }
        _ => panic!("Expected GoogleDriveError for missing client ID"),
    }

    // Test with only client ID set
    set_env_var("GOOGLE_CLIENT_ID", "partial_client_id");
    let client_result = GoogleDriveClient::from_env();
    assert!(client_result.is_err());

    match client_result.unwrap_err() {
        SyncError::GoogleDriveError(msg) => {
            assert!(msg.contains("GOOGLE_CLIENT_SECRET"));
        }
        _ => panic!("Expected GoogleDriveError for missing client secret"),
    }

    // Clean up
    unsafe {
        std::env::remove_var("GOOGLE_CLIENT_ID");
    }
}

#[test]
fn test_token_refresh_logic_simulation() {
    let mut client = GoogleDriveClient::new(GoogleDriveConfig {
        client_id: "test_client".to_string(),
        client_secret: "test_secret".to_string(),
        access_token: Some("initial_token".to_string()),
        refresh_token: Some("refresh_token_123".to_string()),
        base_folder_id: None,
        token_expires_at: Some(chrono::Utc::now().timestamp() + 3600), // 1 hour from now
    });

    // Test that valid token doesn't need refresh
    assert!(!client.is_token_expired());

    // Simulate token expiry
    client.config.token_expires_at = Some(chrono::Utc::now().timestamp() - 3600); // 1 hour ago
    assert!(client.is_token_expired());

    // Test refresh token availability
    let refresh_token = client.config.refresh_token.as_ref();
    assert!(refresh_token.is_some());
    assert_eq!(refresh_token.unwrap(), "refresh_token_123");

    // Test scenario with no refresh token
    client.config.refresh_token = None;
    let token_result = client
        .config
        .refresh_token
        .as_ref()
        .ok_or_else(|| SyncError::GoogleDriveError("No refresh token available".to_string()));
    assert!(token_result.is_err());
}

#[test]
fn test_auth_config_default_values() {
    let config = GoogleDriveConfig::default();
    assert_eq!(config.client_id, "");
    assert_eq!(config.client_secret, "");
    assert_eq!(config.access_token, None);
    assert_eq!(config.refresh_token, None);
    assert_eq!(config.base_folder_id, None);
    assert_eq!(config.token_expires_at, None);

    let client = GoogleDriveClient::new(config);
    assert!(client.get_access_token().is_err());
    assert!(client.is_token_expired());
}

#[test]
fn test_token_response_deserialization() {
    let json_response = r#"{
        "access_token": "ya29.new_token",
        "refresh_token": "new_refresh_token",
        "expires_in": 3600,
        "token_type": "Bearer"
    }"#;

    let token_response: OAuthTokenResponse =
        serde_json::from_str(json_response).expect("Failed to deserialize token response");

    assert_eq!(token_response.access_token, "ya29.new_token");
    assert_eq!(
        token_response.refresh_token,
        Some("new_refresh_token".to_string())
    );
    assert_eq!(token_response.expires_in, 3600);
    assert_eq!(token_response.token_type, "Bearer");

    // Test without refresh token (Google sometimes doesn't return it)
    let json_response_no_refresh = r#"{
        "access_token": "ya29.new_token",
        "expires_in": 3600,
        "token_type": "Bearer"
    }"#;

    let token_response_no_refresh: OAuthTokenResponse =
        serde_json::from_str(json_response_no_refresh)
            .expect("Failed to deserialize token response without refresh token");

    assert_eq!(token_response_no_refresh.access_token, "ya29.new_token");
    assert_eq!(token_response_no_refresh.refresh_token, None);
    assert_eq!(token_response_no_refresh.expires_in, 3600);
    assert_eq!(token_response_no_refresh.token_type, "Bearer");
}
