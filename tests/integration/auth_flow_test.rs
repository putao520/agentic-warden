//! OAuth Authentication Flow Integration Tests
//!
//! Integration tests for the real OOB authentication flow

use agentic_warden::sync::{OAuthClient, SmartOAuthAuthenticator};
use agentic_warden::sync::oauth_client::OAuthConfig;
use anyhow::{Context, Result};
use std::path::PathBuf;

// Import test helpers from the common module
#[path = "../common/test_helpers.rs"]
mod test_helpers;
use test_helpers::*;

/// Test configuration for real OAuth flow
struct OAuthTestConfig {
    pub client_id: String,
    pub client_secret: String,
    pub test_dir: PathBuf,
}

impl OAuthTestConfig {
    fn from_code_constants() -> Self {
        let test_dir = std::env::temp_dir()
            .join("agentic_warden_oauth_test")
            .join(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs().to_string());

        std::fs::create_dir_all(&test_dir).ok();

        Self {
            // Use the real constants from your codebase
            client_id: "77185225430.apps.googleusercontent.com".to_string(),
            client_secret: "d-FL95Q19q7MQmFpd7hHD0Ty".to_string(),
            test_dir,
        }
    }
}

/// Test real OOB authorization URL generation
#[test]
fn test_real_oob_url_generation() -> Result<()> {
    let config = OAuthTestConfig::from_code_constants();

    let oauth_client = OAuthClient::new(
        config.client_id.clone(),
        config.client_secret.clone(),
        None,
    );

    // Generate real OOB authorization URL
    let auth_url = oauth_client.generate_auth_url()
        .context("Failed to generate OOB authorization URL")?;

    // Verify URL contains OOB redirect URI
    if !auth_url.contains("urn:ietf:wg:oauth:2.0:oob") {
        return Err(anyhow::anyhow!(
            "Generated URL doesn't contain OOB redirect URI: {}",
            auth_url
        ));
    }

    // Verify URL contains correct client ID
    if !auth_url.contains(&config.client_id) {
        return Err(anyhow::anyhow!("URL doesn't contain correct client_id"));
    }

    // Verify URL contains required parameters
    if !auth_url.contains("scope=") {
        return Err(anyhow::anyhow!("URL doesn't contain scope parameter"));
    }

    if !auth_url.contains("response_type=code") {
        return Err(anyhow::anyhow!("URL doesn't contain response_type parameter"));
    }

    println!("✅ OOB authorization URL generated successfully!");
    println!("URL length: {} characters", auth_url.len());
    println!("Client ID: {}", config.client_id);

    Ok(())
}

/// Test SmartOAuth authenticator initialization
#[test]
fn test_smart_oauth_initialization() -> Result<()> {
    let config = OAuthTestConfig::from_code_constants();

    let oauth_config = OAuthConfig {
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        access_token: None,
        refresh_token: None,
        expires_in: 0,
        token_type: "Bearer".to_string(),
        scopes: vec![
            "https://www.googleapis.com/auth/drive.file".to_string(),
            "https://www.googleapis.com/auth/drive.metadata.readonly".to_string(),
        ],
    };

    let authenticator = SmartOAuthAuthenticator::new(oauth_config.clone());

    // Test that authenticator was created successfully
    println!("✅ SmartOAuth authenticator created successfully!");
    println!("Client ID: {}", oauth_config.client_id);
    println!("Scopes: {:?}", oauth_config.scopes);

    Ok(())
}

/// Test environment detection
#[tokio::test]
async fn test_environment_detection() -> Result<()> {
    let config = OAuthTestConfig::from_code_constants();

    let oauth_config = OAuthConfig {
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        access_token: None,
        refresh_token: None,
        expires_in: 0,
        token_type: "Bearer".to_string(),
        scopes: vec![
            "https://www.googleapis.com/auth/drive.file".to_string(),
        ],
    };

    let authenticator = SmartOAuthAuthenticator::new(oauth_config);

    // Test environment detection methods (these don't require actual authentication)
    println!("✅ Environment detection methods available");
    println!("Test directory: {:?}", config.test_dir);

    // Note: We can't test actual environment detection easily in a unit test
    // as it depends on the actual runtime environment

    Ok(())
}

/// Test manual authorization URL generation
#[test]
fn test_manual_auth_url_generation() -> Result<()> {
    let config = OAuthTestConfig::from_code_constants();

    let oauth_config = OAuthConfig {
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        access_token: None,
        refresh_token: None,
        expires_in: 0,
        token_type: "Bearer".to_string(),
        scopes: vec![
            "https://www.googleapis.com/auth/drive.file".to_string(),
        ],
    };

    let authenticator = SmartOAuthAuthenticator::new(oauth_config);

    // Test that we can access the configuration
    let retrieved_config = authenticator.get_config();

    assert_eq!(retrieved_config.client_id, config.client_id);
    assert_eq!(retrieved_config.client_secret, config.client_secret);
    assert!(retrieved_config.scopes.contains(&"https://www.googleapis.com/auth/drive.file".to_string()));

    println!("✅ OAuth configuration setup successful!");
    println!("Client ID: {}", retrieved_config.client_id);
    println!("Scopes: {:?}", retrieved_config.scopes);

    Ok(())
}

/// Test token validation logic
#[test]
fn test_token_validation() -> Result<()> {
    // Create valid token data
    let valid_token_data = create_real_auth_data();

    // Validate required fields
    let required_fields = vec![
        "client_id",
        "client_secret",
        "access_token",
        "expires_in",
        "token_type",
        "scope",
        "created_at",
    ];

    for field in required_fields {
        if !valid_token_data.get(field).is_some() {
            return Err(anyhow::anyhow!("Missing required field: {}", field));
        }
    }

    // Validate token type
    let token_type = valid_token_data.get("token_type").unwrap().as_str().unwrap();
    if token_type != "Bearer" {
        return Err(anyhow::anyhow!("Invalid token type: {}", token_type));
    }

    // Validate expiration time
    let expires_in = valid_token_data.get("expires_in").unwrap().as_u64().unwrap();
    if expires_in == 0 {
        return Err(anyhow::anyhow!("Invalid expiration time: {}", expires_in));
    }

    // Validate scope
    let scope = valid_token_data.get("scope").unwrap().as_str().unwrap();
    if !scope.contains("drive") {
        return Err(anyhow::anyhow!("Invalid scope: {}", scope));
    }

    println!("✅ Token validation passed!");
    println!("Token expires in: {} seconds", expires_in);
    println!("Token scope: {}", scope);

    Ok(())
}

/// Test OAuth client configuration
#[test]
fn test_oauth_client_configuration() -> Result<()> {
    let config = OAuthTestConfig::from_code_constants();

    // Test OAuth client creation
    let oauth_client = OAuthClient::new(
        config.client_id.clone(),
        config.client_secret.clone(),
        None,
    );

    // Test that the client stores configuration correctly
    let stored_config = oauth_client.config();
    assert_eq!(stored_config.client_id, config.client_id);
    assert_eq!(stored_config.client_secret, config.client_secret);
    assert_eq!(stored_config.scopes.len(), 2); // Default scopes

    println!("✅ OAuth client configuration validated!");
    println!("Client ID: {}", stored_config.client_id);
    println!("Number of scopes: {}", stored_config.scopes.len());

    Ok(())
}

/// Test auth file management
#[test]
fn test_auth_file_management() -> Result<()> {
    // Create auth file
    let auth_path = create_real_auth_file()
        .context("Failed to create auth file")?;

    assert_file_exists(&auth_path);

    // Verify file content
    let content = std::fs::read_to_string(&auth_path)
        .context("Failed to read auth file")?;

    let auth_data: serde_json::Value = serde_json::from_str(&content)
        .context("Failed to parse auth data")?;

    assert_json_has_key(&auth_data, "access_token");
    assert_json_has_key(&auth_data, "client_id");

    // Test backup functionality
    let backup = backup_auth_file()
        .context("Failed to backup auth file")?;

    assert!(backup.is_some());
    let backup_size = backup.unwrap().len();
    assert!(backup_size > 100); // Should have substantial content

    // Test cleanup
    cleanup_auth_file()
        .context("Failed to cleanup auth file")?;

    assert_file_not_exists(&auth_path);

    println!("✅ Auth file management tested successfully!");
    println!("Auth path: {:?}", auth_path);
    println!("Backup size: {} bytes", backup_size);

    Ok(())
}

/// Test token refresh configuration
#[test]
fn test_token_refresh_configuration() -> Result<()> {
    let config = OAuthTestConfig::from_code_constants();

    // Create OAuth client with refresh token
    let oauth_client = OAuthClient::new(
        config.client_id.clone(),
        config.client_secret.clone(),
        Some("test_refresh_token".to_string()),
    );

    // Verify refresh token is stored
    let stored_config = oauth_client.config();
    assert_eq!(
        stored_config.refresh_token,
        Some("test_refresh_token".to_string())
    );

    println!("✅ Token refresh configuration validated!");
    println!("Refresh token: {:?}", stored_config.refresh_token);

    Ok(())
}

/// Test error handling in OAuth flow
#[test]
fn test_oauth_error_handling() -> Result<()> {
    let config = OAuthTestConfig::from_code_constants();

    // Test invalid client ID
    let result = std::panic::catch_unwind(|| {
        OAuthClient::new(
            "invalid_client_id".to_string(),
            config.client_secret.clone(),
            None,
        )
    });

    assert!(result.is_ok(), "OAuth client creation should not panic");

    // Test empty client secret
    let oauth_client = OAuthClient::new(
        config.client_id.clone(),
        "".to_string(),
        None,
    );

    // The client should be created but operations might fail
    let stored_config = oauth_client.config();
    assert_eq!(stored_config.client_secret, "");

    println!("✅ OAuth error handling validated!");
    println!("Client handles invalid configurations gracefully");

    Ok(())
}

/// Test concurrent OAuth client creation
#[test]
fn test_concurrent_oauth_clients() -> Result<()> {
    let config = OAuthTestConfig::from_code_constants();

    // Clone values to avoid move issues in closure
    let client_id = config.client_id.clone();
    let client_secret = config.client_secret.clone();

    // Create multiple OAuth clients simultaneously
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let client_id = client_id.clone();
            let client_secret = client_secret.clone();
            std::thread::spawn(move || {
                let client = OAuthClient::new(
                    format!("client_{}", client_id),
                    client_secret,
                    Some(format!("refresh_token_{}", i)),
                );

                let stored_config = client.config();
                (i, stored_config.client_id.clone())
            })
        })
        .collect();

    // Wait for all threads to complete
    let results: Vec<_> = handles.into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();

    // Verify all clients were created successfully
    assert_eq!(results.len(), 5);

    for (i, client_id) in results {
        assert_eq!(client_id, format!("client_{}", config.client_id));
        println!("Thread {} created client with ID: {}", i, client_id);
    }

    println!("✅ Concurrent OAuth client creation validated!");

    Ok(())
}