# Real Environment Integration Tests

These tests use your **REAL** Google Drive account for end-to-end testing.

## ⚠️ Important Warnings

- **NO MOCKS**: All operations use real Google Drive APIs
- **REQUIRES AUTH**: Must complete OAuth authentication first  
- **NETWORK REQUIRED**: Tests will upload/download files
- **CREATES FILES**: Tests create temporary files in your Google Drive
- **AUTO-CLEANUP**: Most tests clean up after themselves

## Prerequisites

### 1. Complete OAuth Authentication

Before running any tests, you must authenticate with Google Drive:

```bash
# Run the main application to complete OAuth flow
cargo run --bin agentic-warden

# Follow the OAuth prompts to:
# 1. Enter your Google Client ID
# 2. Enter your Google Client Secret  
# 3. Complete browser authorization
# 4. Verify auth.json is created
```

**Verify authentication:**

```bash
# Check that auth.json exists
ls -la ~/.agentic-warden/auth.json

# On Windows:
dir %USERPROFILE%\.agentic-warden\auth.json

# Verify it contains required fields
cat ~/.agentic-warden/auth.json
```

Expected `auth.json` structure:

```json
{
  "client_id": "your-client-id.apps.googleusercontent.com",
  "client_secret": "your-client-secret",
  "refresh_token": "your-refresh-token",
  "access_token": "current-access-token",
  "expires_at": 1234567890,
  "token_type": "Bearer",
  "scope": "https://www.googleapis.com/auth/drive.file"
}
```

### 2. Network Connectivity

Tests require:
- Internet connection
- Access to `accounts.google.com`
- Access to `www.googleapis.com`

### 3. Google Drive Storage

Tests create small files (<1MB total). Ensure you have:
- At least 10MB free space in Google Drive
- No quota restrictions on Google Drive API

## Test Suites

### 1. OAuth Authentication Tests

**File:** `tests/real_oauth_integration_test.rs`

Tests OAuth credential loading, token refresh, and authentication workflows.

```bash
# Run all OAuth tests
cargo test --test real_oauth_integration_test -- --ignored --nocapture

# Run specific test
cargo test --test real_oauth_integration_test test_load_real_auth_credentials -- --ignored --nocapture
```

**What it tests:**
- ✅ Loading credentials from `~/.agentic-warden/auth.json`
- ✅ OAuth client initialization
- ✅ Token refresh mechanism
- ✅ Access token retrieval
- ✅ Configuration validation
- ✅ Complete OAuth workflow

**Expected output:**
```
🔐 Test: Load Real Auth Credentials
📁 Auth file path: "/home/user/.agentic-warden/auth.json"
✅ Auth file exists
📋 Checking required fields...
  ✅ client_id present
  ✅ client_secret present
  ✅ refresh_token present
  ✅ access_token present
  ✅ expires_at present
✅ Auth credentials loaded successfully
```

### 2. Google Drive Operations Tests

**File:** `tests/real_google_drive_test.rs`

Tests core Google Drive operations: upload, download, list, search, update.

```bash
# Run all Google Drive tests
cargo test --test real_google_drive_test -- --ignored --nocapture

# Run specific tests
cargo test --test real_google_drive_test test_google_drive_upload_and_list -- --ignored --nocapture
cargo test --test real_google_drive_test test_google_drive_upload_download_roundtrip -- --ignored --nocapture
```

**What it tests:**
- ✅ File upload to Google Drive
- ✅ File download from Google Drive
- ✅ Upload/download content verification (roundtrip)
- ✅ Folder creation and management
- ✅ File listing and search
- ✅ File metadata operations
- ✅ File content updates
- ✅ Large file handling (>100KB)

**Expected output:**
```
📤 Test: Upload File and List Files
📁 Created test file: "/tmp/test_config.json"
  Content: {"test": "data", ...}
🔐 Initializing Google Drive service...
  ✅ Google Drive service initialized
📤 Uploading test file: agentic_warden_test_1234567890
  ✅ File uploaded successfully!
    File ID: 1a2b3c4d5e6f7g8h9i0j
    File name: agentic_warden_test_1234567890
    Size: 67 bytes
📋 Listing files to verify upload...
  Found 1 files matching 'agentic_warden_test_':
    - agentic_warden_test_1234567890 (ID: 1a2b3c4d5e6f7g8h9i0j, Size: 67 bytes)
  ✅ File found in search results
🗑️  Cleaning up: Deleting test file...
  ✅ Test file deleted successfully
✅ Upload and list test completed successfully
```

### 3. Push/Pull End-to-End Tests

**File:** `tests/real_push_pull_e2e_test.rs`

Tests complete configuration sync workflows with real Google Drive.

```bash
# Run all push/pull tests
cargo test --test real_push_pull_e2e_test -- --ignored --nocapture

# Run specific workflow test
cargo test --test real_push_pull_e2e_test test_full_push_pull_roundtrip -- --ignored --nocapture
```

**What it tests:**
- ✅ Push local configs to Google Drive (compressed)
- ✅ Pull configs from Google Drive (download & extract)
- ✅ Full roundtrip (push → delete → pull → verify)
- ✅ Push idempotency (no changes = no upload)
- ✅ Multiple directory sync
- ✅ Large directory performance (50+ files)
- ✅ Nested directory structures (4+ levels deep)

**Test workflow:**

```
1. Create local test configurations
   └── app_config.json
   └── settings.toml
   └── .env
   └── nested/
       ├── nested_config.yaml
       └── credentials.txt

2. Push to Google Drive
   └── Compress to TAR.GZ
   └── Upload to agentic-warden/test_configs_XXX/

3. Delete local files
   └── Simulate data loss

4. Pull from Google Drive
   └── Download TAR.GZ
   └── Extract to original location

5. Verify restoration
   └── All files restored
   └── Content matches exactly
   └── Directory structure preserved
```

**Expected output:**
```
🔄 Test: Full Push/Pull Roundtrip
📋 Step 1: Creating test configuration directory...
  ✅ Created: "/tmp/test_configs_1234567890"
  📁 Total files: 5

📋 Step 2: Initializing sync manager...
  🔐 Authenticating with Google Drive...
  ✅ Authenticated

📋 Step 3: Pushing to Google Drive...
  ✅ Push completed!
    Directory: test_configs_1234567890
    Changed: true
    Uploaded: true
    Archive size: 2048 bytes (2.00 KB)

📋 Step 4: Deleting local config (simulating data loss)...
  ✅ Local config deleted

📋 Step 5: Pulling from Google Drive to restore...
  ✅ Pull completed!
    Directory: test_configs_1234567890
    Changed: true
    Downloaded size: 2048 bytes

📋 Step 6: Verifying restored files...
  ✅ Found: app_config.json
  ✅ Found: settings.toml
  ✅ Found: .env
  ✅ Found: nested/nested_config.yaml
  ✅ Found: nested/credentials.txt
  ✅ File count matches: 5

📋 Step 7: Verifying file contents...
  ✅ app_config.json content verified
  ✅ settings.toml content verified
  ✅ .env content verified
  ✅ nested_config.yaml content verified

✅ Full Push/Pull roundtrip test completed successfully!
```

### 4. Cleanup Utility Tests

**File:** `tests/real_cleanup_test.rs`

Utilities for cleaning up test artifacts from Google Drive.

```bash
# List test files (safe, read-only)
cargo test --test real_cleanup_test test_list_test_files -- --ignored --nocapture

# List test folders (safe, read-only)
cargo test --test real_cleanup_test test_list_test_folders -- --ignored --nocapture

# Full cleanup report
cargo test --test real_cleanup_test test_full_cleanup_report -- --ignored --nocapture

# Delete test files (destructive, 5s delay)
cargo test --test real_cleanup_test test_cleanup_test_files -- --ignored --nocapture

# Delete test folders (destructive, 5s delay)
cargo test --test real_cleanup_test test_cleanup_test_folders -- --ignored --nocapture

# Delete entire agentic-warden folder (very destructive, 10s delay)
cargo test --test real_cleanup_test test_cleanup_agentic_warden_folder -- --ignored --nocapture
```

**What it cleans:**

Test files are identified by these prefixes:
- `agentic_warden_test_*` - Individual test files
- `test_configs_*` - Push/pull test directories
- `idempotent_test_*` - Idempotency test directories
- `dir1_*`, `dir2_*`, `dir3_*` - Multi-directory tests
- `large_config_*` - Large directory tests
- `nested_test_*` - Nested structure tests

**Safety features:**
- ⏰ Countdown delay before destructive operations (5-10 seconds)
- 📋 Lists files before deletion
- ✅ Confirmation of each deletion
- 📊 Summary report of cleanup results

## Running Tests

### Run All Tests

```bash
# Run everything (OAuth → Drive → Push/Pull)
cargo test --test real_oauth_integration_test \
            --test real_google_drive_test \
            --test real_push_pull_e2e_test \
            -- --ignored --nocapture
```

### Run by Category

```bash
# OAuth tests only
cargo test --test real_oauth_integration_test -- --ignored --nocapture

# Google Drive tests only
cargo test --test real_google_drive_test -- --ignored --nocapture

# Push/Pull tests only
cargo test --test real_push_pull_e2e_test -- --ignored --nocapture

# Cleanup utilities
cargo test --test real_cleanup_test -- --ignored --nocapture
```

### Run Specific Test

```bash
# Example: Run just the roundtrip test
cargo test --test real_push_pull_e2e_test test_full_push_pull_roundtrip -- --ignored --nocapture
```

### Parallel vs Sequential

By default, Cargo runs tests in parallel. For these integration tests, you may want to run sequentially:

```bash
# Run tests one at a time
cargo test --test real_oauth_integration_test -- --ignored --nocapture --test-threads=1
```

## Test Timing Estimates

Based on network conditions and Google Drive API response times:

| Test Suite | Test Count | Est. Time | Network |
|------------|------------|-----------|---------|
| OAuth Tests | 7 | 10-30s | ⬇️ Low |
| Google Drive Tests | 8 | 30-90s | ⬇️⬆️ Medium |
| Push/Pull Tests | 6 | 60-180s | ⬇️⬆️ High |
| Cleanup Tests | 6 | 10-60s | ⬇️⬆️ Varies |
| **Total** | **27** | **~2-6 min** | |

Factors affecting timing:
- Network speed and latency
- Google Drive API rate limits
- File sizes (larger in performance tests)
- Current Google Drive load

## Troubleshooting

### Error: Auth file not found

```
Auth file not found at "/home/user/.agentic-warden/auth.json"
Please run OAuth authentication flow first.
```

**Solution:**
```bash
cargo run --bin agentic-warden
# Complete OAuth authentication
```

### Error: Token expired / Invalid credentials

```
Token refresh failed: invalid_grant
```

**Solution:**
1. Delete old auth file:
   ```bash
   rm ~/.agentic-warden/auth.json
   ```
2. Re-authenticate:
   ```bash
   cargo run --bin agentic-warden
   ```

### Error: Network connection failed

```
Failed to connect to accounts.google.com
```

**Solution:**
- Check internet connectivity
- Verify firewall settings
- Check proxy configuration
- Try again (may be temporary Google API issue)

### Error: Quota exceeded

```
Failed to upload file: quotaExceeded
```

**Solution:**
- Wait for API quota to reset (usually hourly)
- Free up space in Google Drive
- Check Google Cloud Console for quota limits

### Error: Rate limit exceeded

```
Failed: rateLimitExceeded
```

**Solution:**
- Tests may have run too quickly
- Wait 1-5 minutes
- Run tests with `--test-threads=1` for sequential execution

### Error: Test file already exists

```
File conflict: agentic_warden_test_XXXXX already exists
```

**Solution:**
```bash
# Clean up old test files first
cargo test --test real_cleanup_test test_cleanup_test_files -- --ignored --nocapture
```

## Best Practices

### Before Testing

1. ✅ Authenticate with Google Drive
2. ✅ Verify `auth.json` exists and is valid
3. ✅ Check network connectivity
4. ✅ Have at least 10MB free in Google Drive

### During Testing

1. 📝 Use `--nocapture` to see detailed output
2. ⏱️ Be patient - tests involve network I/O
3. 🔍 Monitor test output for issues
4. ⚠️ Don't interrupt destructive cleanup tests

### After Testing

1. 🧹 Run cleanup tests to remove artifacts
2. 📊 Review test output for any failures
3. 🔐 Keep `auth.json` secure
4. 📝 Report any issues on GitHub

## CI/CD Integration

These tests are marked with `#[ignore]` and will **NOT** run in standard CI/CD pipelines.

To run in CI/CD (not recommended):

```yaml
# Example GitHub Actions (NOT RECOMMENDED for real API tests)
- name: Run Integration Tests
  env:
    # Would need to inject real credentials (UNSAFE)
    GOOGLE_CLIENT_ID: ${{ secrets.GOOGLE_CLIENT_ID }}
    GOOGLE_CLIENT_SECRET: ${{ secrets.GOOGLE_CLIENT_SECRET }}
    GOOGLE_REFRESH_TOKEN: ${{ secrets.GOOGLE_REFRESH_TOKEN }}
  run: |
    cargo test --test real_oauth_integration_test -- --ignored
```

**Recommendation:** Use mock tests for CI/CD, real tests for local development only.

## Security Notes

### Credential Security

- 🔒 `auth.json` contains sensitive OAuth tokens
- 🚫 Never commit `auth.json` to version control
- 🔐 Keep `.agentic-warden/` directory private
- 🗑️ Revoke access in Google Cloud Console after testing

### Access Permissions

Tests use these Google Drive API scopes:
- `https://www.googleapis.com/auth/drive.file` - Access to files created by this app only

Tests **CANNOT**:
- ❌ Access your personal files
- ❌ Read files created by other apps
- ❌ Access files outside `agentic-warden/` folder

### Revoking Access

To revoke test access:

1. Visit: https://myaccount.google.com/permissions
2. Find "agentic-warden" application
3. Click "Remove Access"
4. Delete local `~/.agentic-warden/auth.json`

## Contributing

When adding new real integration tests:

1. ✅ Mark with `#[ignore]` attribute
2. ✅ Use `TEST_FILE_PREFIX` for test artifacts
3. ✅ Implement cleanup in the test or cleanup utility
4. ✅ Add detailed output with emoji indicators
5. ✅ Document in this README
6. ✅ Handle errors gracefully
7. ✅ Add timing information for performance tests

## Support

For issues with tests:

1. Check this README first
2. Verify prerequisites are met
3. Run cleanup tests
4. Check GitHub Issues: https://github.com/putao520/agentic-warden/issues
5. Open new issue with:
   - Test command run
   - Error message
   - Environment (OS, Rust version)
   - Network conditions

---

**Last Updated:** 2025-10-30  
**Test Framework:** Rust + Tokio  
**Google Drive API:** v3  
**OAuth Version:** 2.0
