//! Integration tests for Google Drive OAuth functionality
//!
//! These tests verify the OAuth authentication flow and token management
//! using mock responses and simulated scenarios.

use agentic_warden::sync::error::SyncError;
use agentic_warden::sync::google_drive_client::{
    GoogleDriveClient, GoogleDriveConfig, OAuthTokenResponse,
};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_oauth_url_generation() {
    let config = GoogleDriveConfig {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: None,
        refresh_token: None,
        base_folder_id: None,
        token_expires_at: None,
    };

    let client = GoogleDriveClient::new(config);

    let auth_url = client
        .generate_auth_url()
        .expect("Failed to generate auth URL");

    // Verify URL contains required components
    assert!(auth_url.starts_with("https://accounts.google.com/o/oauth2/v2/auth"));
    assert!(auth_url.contains("client_id=test_client_id"));
    assert!(auth_url.contains("redirect_uri=urn%3Aietf%3Awg%3Aoauth%3A2.0%3Aoob"));
    assert!(auth_url.contains("response_type=code"));
    assert!(auth_url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.file"));
    assert!(auth_url.contains("access_type=offline"));
    assert!(auth_url.contains("prompt=consent"));
}

#[tokio::test]
async fn test_token_expiry_detection() {
    let config = GoogleDriveConfig {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: Some("test_access_token".to_string()),
        refresh_token: Some("test_refresh_token".to_string()),
        base_folder_id: None,
        token_expires_at: Some(chrono::Utc::now().timestamp() - 3600), // Expired 1 hour ago
    };

    let client = GoogleDriveClient::new(config);

    // Test expired token detection
    assert!(
        client.is_token_expired(),
        "Token should be detected as expired"
    );
}

#[tokio::test]
async fn test_token_validity() {
    let config = GoogleDriveConfig {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: Some("test_access_token".to_string()),
        refresh_token: Some("test_refresh_token".to_string()),
        base_folder_id: None,
        token_expires_at: Some(chrono::Utc::now().timestamp() + 3600), // Expires in 1 hour
    };

    let client = GoogleDriveClient::new(config);

    // Test valid token detection
    assert!(
        !client.is_token_expired(),
        "Token should be detected as valid"
    );
}

#[tokio::test]
async fn test_token_expiry_without_timestamp() {
    let config = GoogleDriveConfig {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: Some("test_access_token".to_string()),
        refresh_token: Some("test_refresh_token".to_string()),
        base_folder_id: None,
        token_expires_at: None, // No expiry timestamp
    };

    let client = GoogleDriveClient::new(config);

    // Test that token without expiry info is considered expired
    assert!(
        client.is_token_expired(),
        "Token without expiry should be considered expired"
    );
}

#[test]
fn test_auth_config_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let auth_path = temp_dir.path().join("auth.json");

    // Create a custom config for testing
    let test_config = GoogleDriveConfig {
        client_id: "test_client_id_123".to_string(),
        client_secret: "test_client_secret_456".to_string(),
        access_token: Some("test_access_token_789".to_string()),
        refresh_token: Some("test_refresh_token_abc".to_string()),
        base_folder_id: Some("test_folder_id_xyz".to_string()),
        token_expires_at: Some(1234567890),
    };

    let client = GoogleDriveClient::new(test_config);

    // Save configuration
    client
        .save_auth_config()
        .expect("Failed to save auth config");

    // Verify file was created
    assert!(auth_path.exists(), "Auth config file should exist");

    // Load configuration
    let loaded_config = GoogleDriveClient::load_auth_config()
        .expect("Failed to load auth config")
        .expect("Should have loaded config");

    // Verify all fields were saved and loaded correctly
    assert_eq!(loaded_config.client_id, "test_client_id_123");
    assert_eq!(loaded_config.client_secret, "test_client_secret_456");
    assert_eq!(
        loaded_config.access_token,
        Some("test_access_token_789".to_string())
    );
    assert_eq!(
        loaded_config.refresh_token,
        Some("test_refresh_token_abc".to_string())
    );
    assert_eq!(
        loaded_config.base_folder_id,
        Some("test_folder_id_xyz".to_string())
    );
    assert_eq!(loaded_config.token_expires_at, Some(1234567890));
}

#[test]
fn test_auth_config_default_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create client with custom auth path
    let auth_path = temp_dir.path().join("nonexistent_auth.json");

    // Load should return None for non-existent file
    let loaded_config = GoogleDriveClient::load_auth_config().expect("Failed to load auth config");
    assert!(
        loaded_config.is_none(),
        "Should return None for non-existent config"
    );
}

#[test]
fn test_auth_config_backward_compatibility() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let auth_path = temp_dir.path().join("old_auth.json");

    // Create auth config in old format (without token_expires_at)
    let old_config = json!({
        "client_id": "old_client_id",
        "client_secret": "old_client_secret",
        "access_token": "old_access_token",
        "refresh_token": "old_refresh_token",
        "base_folder_id": "old_folder_id"
    });

    fs::write(&auth_path, old_config.to_string()).expect("Failed to write old format config");

    // Manually load and parse to test backward compatibility
    let content = fs::read_to_string(&auth_path).expect("Failed to read old config file");

    let parsed_config: GoogleDriveConfig =
        serde_json::from_str(&content).expect("Failed to parse old config format");

    // Verify fields are loaded correctly
    assert_eq!(parsed_config.client_id, "old_client_id");
    assert_eq!(parsed_config.client_secret, "old_client_secret");
    assert_eq!(
        parsed_config.access_token,
        Some("old_access_token".to_string())
    );
    assert_eq!(
        parsed_config.refresh_token,
        Some("old_refresh_token".to_string())
    );
    assert_eq!(
        parsed_config.base_folder_id,
        Some("old_folder_id".to_string())
    );
    assert_eq!(parsed_config.token_expires_at, None); // Should be None for old format
}

#[tokio::test]
async fn test_oauth_token_response_parsing() {
    // Test token response with refresh token
    let token_json = json!({
        "access_token": "ya29.new_access_token",
        "refresh_token": "new_refresh_token",
        "expires_in": 3600,
        "token_type": "Bearer"
    });

    let token_response: OAuthTokenResponse =
        serde_json::from_value(token_json).expect("Failed to parse token response");

    assert_eq!(token_response.access_token, "ya29.new_access_token");
    assert_eq!(
        token_response.refresh_token,
        Some("new_refresh_token".to_string())
    );
    assert_eq!(token_response.expires_in, 3600);
    assert_eq!(token_response.token_type, "Bearer");
}

#[tokio::test]
async fn test_oauth_token_response_without_refresh_token() {
    // Test token response without refresh token (some OAuth flows don't return it)
    let token_json = json!({
        "access_token": "ya29.another_access_token",
        "expires_in": 1800,
        "token_type": "Bearer"
    });

    let token_response: OAuthTokenResponse = serde_json::from_value(token_json)
        .expect("Failed to parse token response without refresh token");

    assert_eq!(token_response.access_token, "ya29.another_access_token");
    assert_eq!(token_response.refresh_token, None); // Should be None
    assert_eq!(token_response.expires_in, 1800);
    assert_eq!(token_response.token_type, "Bearer");
}

#[test]
fn test_default_config_values() {
    let default_config = GoogleDriveConfig::default();

    assert_eq!(default_config.client_id, "");
    assert_eq!(default_config.client_secret, "");
    assert_eq!(default_config.access_token, None);
    assert_eq!(default_config.refresh_token, None);
    assert_eq!(default_config.base_folder_id, None);
    assert_eq!(default_config.token_expires_at, None);
}

#[test]
fn test_access_token_retrieval() {
    // Test with valid access token
    let config_with_token = GoogleDriveConfig {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: Some("valid_access_token".to_string()),
        refresh_token: Some("valid_refresh_token".to_string()),
        base_folder_id: None,
        token_expires_at: None,
    };

    let client = GoogleDriveClient::new(config_with_token);
    let token = client.get_access_token().expect("Should have access token");
    assert_eq!(token, "valid_access_token");

    // Test without access token
    let config_without_token = GoogleDriveConfig {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: None,
        refresh_token: Some("valid_refresh_token".to_string()),
        base_folder_id: None,
        token_expires_at: None,
    };

    let client_without_token = GoogleDriveClient::new(config_without_token);
    let result = client_without_token.get_access_token();
    assert!(result.is_err());
    match result.unwrap_err() {
        SyncError::AuthenticationRequired => {} // Expected error
        _ => panic!("Expected AuthenticationRequired error"),
    }
}

#[tokio::test]
async fn test_token_refresh_preparation() {
    let config = GoogleDriveConfig {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: Some("expired_token".to_string()),
        refresh_token: Some("valid_refresh_token".to_string()),
        base_folder_id: None,
        token_expires_at: Some(chrono::Utc::now().timestamp() - 3600), // Expired
    };

    let client = GoogleDriveClient::new(config);

    // Test that refresh would be attempted (we can't actually make the request in tests)
    // but we can verify the preparation logic
    assert!(
        client.is_token_expired(),
        "Token should be detected as expired"
    );
    assert!(
        client.config.refresh_token.is_some(),
        "Should have refresh token available"
    );
}

#[test]
fn test_error_scenarios() {
    // Test invalid auth URL generation with empty client_id
    let config = GoogleDriveConfig {
        client_id: "".to_string(), // Empty client_id should still generate URL
        client_secret: "test_client_secret".to_string(),
        access_token: None,
        refresh_token: None,
        base_folder_id: None,
        token_expires_at: None,
    };

    let client = GoogleDriveClient::new(config);

    // URL generation should still work even with empty client_id
    let result = client.generate_auth_url();
    assert!(
        result.is_ok(),
        "Should generate URL even with empty client_id"
    );

    let url = result.unwrap();
    assert!(
        url.contains("client_id="),
        "URL should contain client_id parameter"
    );
}

#[test]
fn test_multiple_token_expiry_scenarios() {
    let now = chrono::Utc::now().timestamp();

    let test_cases = vec![
        ("already_expired", now - 3600, true),
        ("expires_soon", now + 300, true), // Within 5 minute buffer
        ("still_valid", now + 3600, false),
        ("expires_at_boundary", now + 301, true), // Just outside buffer
        ("expires_far_future", now + 86400, false), // 24 hours from now
    ];

    for (name, expires_at, should_be_expired) in test_cases {
        let config = GoogleDriveConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            access_token: Some("test_token".to_string()),
            refresh_token: Some("test_refresh".to_string()),
            base_folder_id: None,
            token_expires_at: Some(expires_at),
        };

        let client = GoogleDriveClient::new(config);
        let is_expired = client.is_token_expired();

        assert_eq!(
            is_expired, should_be_expired,
            "Token expiry test '{}' failed: expected {}, got {}",
            name, should_be_expired, is_expired
        );
    }
}
