# Google APIs Migration to Official Libraries

## Overview

The agentic-warden project has been migrated from custom Google API implementations to Google's official Rust libraries. This change improves reliability, security, and maintainability.

## What Changed

### 🔄 **Migration Summary**

| Component | Old Implementation | New Implementation |
|-----------|-------------------|-------------------|
| **OAuth 2.0** | Custom HTTP calls to OAuth endpoints | `yup-oauth2` official library |
| **Google Drive** | Manual HTTP requests to Drive API | `google-apis-drive3` official client |
| **HTTP Client** | `reqwest` with manual error handling | `hyper` with built-in retry logic |
| **Token Management** | Manual token refresh and expiry handling | Automatic token refresh with secure storage |

### 📦 **New Dependencies**

```toml
# Google Official APIs
yup-oauth2 = "8.3"                    # OAuth 2.0 authentication
google-apis-common = "6.0"             # Common Google API utilities
google-apis-drive3 = "5.0"             # Google Drive API client
google-apis-oauth2 = "5.0"             # Google OAuth2 API
hyper = "0.14"                         # HTTP client
hyper-rustls = "0.24"                  # TLS support
mime_guess = "2.0"                     # MIME type detection

# Additional utilities
tracing = "0.1"                        # Structured logging
parking_lot = "0.12"                   # High-performance synchronization
```

## Benefits

### 🛡️ **Security Improvements**

- **Secure Token Storage**: Tokens are stored securely with proper encryption
- **Automatic Refresh**: No more manual token expiry handling
- **Scope Validation**: Proper OAuth scope validation and management
- **HTTPS Enforcement**: All API calls use secure connections

### ⚡ **Performance Improvements**

- **Connection Pooling**: Efficient HTTP connection reuse
- **Automatic Retry**: Built-in retry logic for transient failures
- **Streaming**: Large files use streaming to avoid memory issues
- **Parallel Operations**: Better support for concurrent API calls

### 🔧 **Maintainability**

- **Official Support**: Libraries are actively maintained by Google
- **API Completeness**: Access to all Drive API features
- **Standardized Errors**: Consistent error handling across operations
- **Documentation**: Comprehensive official documentation

## Usage Examples

### 🔐 **OAuth Authentication**

```rust
use agentic_warden::sync::OAuthClient;

// Create OAuth client from environment variables
let mut oauth_client = OAuthClient::from_env()?;

// Get access token (automatic refresh if needed)
let access_token = oauth_client.access_token().await?;

// Generate auth URL for manual flow
let auth_url = oauth_client.generate_auth_url()?;

// Exchange authorization code for tokens
let token_response = oauth_client.exchange_code_for_tokens(&auth_code).await?;
```

### 📁 **Google Drive Operations**

```rust
use agentic_warden::sync::{OAuthClient, GoogleDriveService};

// Create authenticated Drive service
let mut oauth_client = OAuthClient::from_env()?;
let drive_service = GoogleDriveService::new(&mut oauth_client).await?;

// Create folder
let folder_id = drive_service.create_folder("my-test-folder").await?;

// Upload file
let file_info = drive_service.upload_file(&file_path, Some(&folder_id)).await?;

// Download file content
let content = drive_service.download_file_content(&file_info.id).await?;

// Search files
let results = drive_service.search_files("name contains '.conf'").await?;

// List folder contents
let files = drive_service.list_folder_files(&folder_id).await?;
```

### 🔄 **Migration from Old API**

```rust
// OLD (Custom Implementation)
use agentic_warden::sync::google_drive_client::{GoogleDriveClient, GoogleDriveConfig};

let mut client = GoogleDriveClient::new(config);
client.authenticate().await?;
let file = client.upload_file(&path, &folder_id).await?;

// NEW (Official Libraries)
use agentic_warden::sync::{OAuthClient, GoogleDriveService};

let mut oauth_client = OAuthClient::new(client_id, client_secret, Some(refresh_token));
let drive_service = GoogleDriveService::new(&mut oauth_client).await?;
let file = drive_service.upload_file(&path, Some(&folder_id)).await?;
```

## Configuration

### Environment Variables

```bash
# Required for OAuth authentication
export GOOGLE_CLIENT_ID="your_google_oauth_client_id"
export GOOGLE_CLIENT_SECRET="your_google_oauth_client_secret"
export GOOGLE_REFRESH_TOKEN="your_google_refresh_token"

# Optional: For testing
export AGENTIC_WARDEN_REAL_TESTS="true"
export AGENTIC_WARDEN_KEEP_TEST_BACKUPS="true"
```

### OAuth Setup

1. **Create Google Cloud Project**:
   ```bash
   # Visit: https://console.cloud.google.com/
   # Create new project or select existing one
   ```

2. **Enable Drive API**:
   ```bash
   # Navigate to: APIs & Services > Library
   # Search and enable "Google Drive API"
   ```

3. **Create OAuth Credentials**:
   ```bash
   # Navigate to: APIs & Services > Credentials
   # Create "OAuth 2.0 Client ID"
   # Select "Desktop app" type
   # Set redirect URIs: http://localhost:8080/oauth/callback
   ```

4. **Generate Refresh Token**:
   ```bash
   # Use OAuth 2.0 Playground: https://developers.google.com/oauthplayground/
   # Or use the provided OAuth authentication flow
   ```

## Testing

### Mock Tests (No Credentials Required)

```bash
# Run mock tests
cargo test oauth_mock_test
cargo test sync_isolated_test
```

### Real Integration Tests (Credentials Required)

```bash
# Set environment variables
export GOOGLE_CLIENT_ID="your_client_id"
export GOOGLE_CLIENT_SECRET="your_client_secret"
export GOOGLE_REFRESH_TOKEN="your_refresh_token"

# Run real integration tests
cargo test oauth_integration_test
cargo test google_drive_integration_test
cargo test sync_workflow_integration_test
```

### Test Categories

| Test Type | Description | Credentials Required |
|-----------|-------------|---------------------|
| **Mock Tests** | Simulated API responses | No |
| **OAuth Integration** | Real OAuth token operations | Yes |
| **Drive API** | Real Drive file operations | Yes |
| **Sync Workflows** | End-to-end sync processes | Yes |

## Troubleshooting

### Common Issues

#### 1. **Authentication Failed**

```
Error: GOOGLE_CLIENT_ID environment variable not set
```

**Solution**: Set the required environment variables
```bash
export GOOGLE_CLIENT_ID="your_client_id"
export GOOGLE_CLIENT_SECRET="your_client_secret"
```

#### 2. **Invalid OAuth Credentials**

```
Error: invalid_client
```

**Solution**: Verify your OAuth configuration in Google Cloud Console

#### 3. **Insufficient Permissions**

```
Error: insufficient
```

**Solution**: Ensure OAuth consent screen has required Drive API scopes

#### 4. **Rate Limiting**

```
Error: userRateLimitExceeded
```

**Solution**: The new libraries handle rate limiting automatically with retry logic

### Debug Mode

Enable detailed logging to troubleshoot issues:

```bash
RUST_LOG=debug cargo test integration_test -- --nocapture
```

### API Quotas

Google Drive API has usage quotas:
- **Read operations**: ~100 requests per 100 seconds
- **Write operations**: ~10 requests per 100 seconds  
- **Daily upload**: ~1GB per day

The new implementation optimizes API usage through:
- Connection pooling
- Automatic retry with exponential backoff
- Batch operations where possible
- Efficient pagination handling

## Backward Compatibility

During the migration period, both implementations are available:

### Old Implementation (Deprecated)
```rust
use agentic_warden::sync::google_drive_client::GoogleDriveClient;
```

### New Implementation (Recommended)
```rust
use agentic_warden::sync::{OAuthClient, GoogleDriveService};
```

The old implementation remains available for existing code but should be migrated to the new implementation for future updates.

## Performance Comparison

| Metric | Old Implementation | New Implementation |
|--------|-------------------|-------------------|
| **Token Refresh** | Manual, error-prone | Automatic, reliable |
| **Connection Handling** | New connection per request | Connection pooling |
| **Error Recovery** | Basic retry logic | Exponential backoff |
| **Large File Upload** | Memory-intensive | Streaming support |
| **Concurrent Operations** | Limited | Better support |

## Future Enhancements

The new implementation enables several future improvements:

1. **Real-time Sync**: Use Drive API change notifications
2. **Version Control**: Leverage Drive's file versioning
3. **Collaboration**: Implement sharing and permissions
4. **Advanced Search**: Full Drive search capabilities
5. **Batch Operations**: Process multiple files efficiently

## Support

For issues related to:

- **Google API Usage**: [Google Drive API Documentation](https://developers.google.com/drive/api)
- **OAuth Setup**: [Google OAuth 2.0 Documentation](https://developers.google.com/identity/protocols/oauth2)
- **Library Issues**: [yup-oauth2 GitHub](https://github.com/dermesser/yup-oauth2)
- **Project Issues**: [agentic-warden Issues](https://github.com/putao520/agentic-warden/issues)

## Migration Checklist

- [ ] Update dependencies in `Cargo.toml`
- [ ] Replace old imports with new ones
- [ ] Update OAuth client initialization
- [ ] Update Drive service calls
- [ ] Test with real Google credentials
- [ ] Update error handling
- [ ] Update documentation
- [ ] Remove old dependencies (optional)

## Conclusion

This migration significantly improves the reliability and maintainability of the agentic-warden project's Google API integration. The official libraries provide better security, performance, and feature support while maintaining backward compatibility during the transition period.