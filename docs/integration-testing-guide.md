# Integration Testing Guide for Agentic-Warden

This comprehensive guide covers real integration testing for the agentic-warden project using authentic Google OAuth and Drive API.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Prerequisites](#prerequisites)
4. [Setup Guide](#setup-guide)
5. [Running Tests](#running-tests)
6. [Test Categories](#test-categories)
7. [Safety Mechanisms](#safety-mechanisms)
8. [CI/CD Integration](#cicd-integration)
9. [Troubleshooting](#troubleshooting)
10. [Best Practices](#best-practices)

## Overview

The agentic-warden integration testing suite provides comprehensive testing of real-world scenarios:

- **Real Google OAuth 2.0 Authentication**: Complete authentication flow with Google's servers
- **Google Drive API Integration**: Actual file operations, folder management, and API interactions
- **End-to-End Sync Workflows**: Complete configuration packing, uploading, downloading, and extraction
- **Safety and Isolation**: Comprehensive safety mechanisms to protect user data
- **CI/CD Integration**: Automated testing with GitHub Actions

### Key Features

- **Security-First Design**: Multiple layers of protection for user data
- **Real API Testing**: Authentic interactions with Google services
- **Comprehensive Coverage**: Tests for success, failure, and edge cases
- **Automatic Cleanup**: Safe cleanup of test artifacts
- **Dry Run Mode**: Testing without actual API calls
- **Cross-Platform Support**: Works on Linux, macOS, and Windows

## Architecture

### Test Architecture

```
Integration Tests
├── Configuration Layer
│   ├── IntegrationTestConfig
│   ├── Environment Variables
│   └── Safety Settings
├── Safety Layer
│   ├── Backup Management
│   ├── Cleanup Registry
│   └── Emergency Procedures
├── OAuth Layer
│   ├── Real Authentication Flow
│   ├── Token Management
│   └── Credential Storage
├── Google Drive Layer
│   ├── File Operations
│   ├── Folder Management
│   └── API Error Handling
├── Sync Workflow Layer
│   ├── Configuration Packing
│   ├── Compression & Upload
│   ├── Download & Extraction
│   └── Conflict Resolution
└── Test Execution Layer
    ├── Test Isolation
    ├── Rate Limiting
    └── Result Reporting
```

### Safety Isolation

- **Namespace Prefixing**: All test resources use unique prefixes (e.g., `test_20241029_123456_`)
- **Temporary Directories**: All test data in isolated temporary directories
- **User Config Backup**: Automatic backup and restoration of user configurations
- **Cleanup Registry**: Comprehensive tracking of all created resources
- **Emergency Stop**: Immediate cleanup capability for critical situations

## Prerequisites

### System Requirements

- **Rust**: Latest stable version
- **Operating System**: Linux, macOS, or Windows
- **Memory**: At least 2GB RAM
- **Disk Space**: 1GB free space for test artifacts
- **Network**: Internet connection for Google API access

### Required Dependencies

```bash
# Rust toolchain
rustup update stable

# System dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y pkg-config libssl-dev

# System dependencies (macOS)
brew install openssl

# System dependencies (Windows)
# No additional dependencies required
```

### Google Cloud Setup

1. **Create Google Cloud Project**
   - Go to [Google Cloud Console](https://console.cloud.google.com/)
   - Create a new project or select existing one
   - Enable billing (required for Google Drive API)

2. **Enable APIs**
   ```
   - Google Drive API
   - OAuth 2.0 API
   ```

3. **Create OAuth Credentials**
   - Go to "APIs & Services" > "Credentials"
   - Click "Create Credentials" > "OAuth client ID"
   - Select "Desktop application"
   - Set authorized redirect URIs to: `urn:ietf:wg:oauth:2.0:oob`
   - Copy Client ID and Client Secret

## Setup Guide

### 1. Environment Configuration

Create a `.env` file in the project root:

```bash
# Enable integration tests
AGENTIC_WARDEN_RUN_INTEGRATION_TESTS=true

# OAuth Test Credentials
AGENTIC_WARDEN_TEST_CLIENT_ID=your_google_oauth_client_id
AGENTIC_WARDEN_TEST_CLIENT_SECRET=your_google_oauth_client_secret
AGENTIC_WARDEN_TEST_REDIRECT_URI=urn:ietf:wg:oauth:2.0:oob

# Test Configuration
AGENTIC_WARDEN_TEST_NAMESPACE_PREFIX=test_local_
AGENTIC_WARDEN_TEST_BASE_FOLDER=agentic-warden-integration-tests
AGENTIC_WARDEN_TEST_LOG_LEVEL=info
AGENTIC_WARDEN_TEST_VERBOSE=true

# Safety Settings
AGENTIC_WARDEN_TEST_MAX_FILES_TOTAL=100
AGENTIC_WARDEN_TEST_CLEANUP_ON_SUCCESS=true
AGENTIC_WARDEN_TEST_CLEANUP_ON_FAILURE=true
AGENTIC_WARDEN_TEST_BACKUP_CONFIG=true

# Rate Limiting
AGENTIC_WARDEN_TEST_RATE_LIMIT_DELAY_MS=2000
AGENTIC_WARDEN_TEST_MAX_FILE_SIZE=10485760  # 10MB
```

### 2. Initial Authentication

Run the OAuth authentication flow once to establish credentials:

```bash
# Run OAuth flow simulation
cargo test --test oauth_integration_test test_real_oauth_complete_flow_simulation -- --ignored

# Or run with configuration verification
cargo test --test oauth_integration_test test_oauth_configuration_verification
```

### 3. Verification

Verify that your setup is working:

```bash
# Check configuration
cargo test --test integration_config

# Verify OAuth setup
cargo test --test oauth_integration_test test_oauth_configuration_verification

# Check Drive API access
cargo test --test google_drive_integration_test test_drive_configuration_verification
```

## Running Tests

### Local Testing

#### Run All Tests
```bash
# Mock tests (always safe to run)
cargo test --test oauth_mock_test
cargo test --test sync_isolated_test

# Real integration tests (requires setup)
cargo test --test oauth_integration_test -- --ignored
cargo test --test google_drive_integration_test -- --ignored
cargo test --test sync_workflow_integration_test -- --ignored
```

#### Run Specific Test Categories
```bash
# OAuth tests only
cargo test --test oauth_integration_test -- --ignored

# Google Drive API tests only
cargo test --test google_drive_integration_test -- --ignored

# Sync workflow tests only
cargo test --test sync_workflow_integration_test -- --ignored

# Safety tests only
cargo test --test test_safety
```

#### Dry Run Mode
```bash
# Run tests without real API calls
export AGENTIC_WARDEN_TEST_DRY_RUN=true
cargo test --test google_drive_integration_test -- --ignored
```

#### Verbose Output
```bash
export AGENTIC_WARDEN_TEST_VERBOSE=true
export RUST_LOG=debug
cargo test --test integration_name -- --nocapture
```

### CI/CD Testing

The project includes GitHub Actions workflows for automated testing:

```bash
# Manual workflow dispatch
# Go to GitHub Actions > Integration Tests > Run workflow

# Scheduled runs (daily at 2 AM UTC)
# Automatic runs on master/develop branches
# Automatic runs on PRs touching integration test files
```

## Test Categories

### 1. Configuration Tests (`integration_config.rs`)

**Purpose**: Test configuration loading, validation, and environment setup.

**Tests**:
- Configuration validation
- Environment variable loading
- Test namespace generation
- Backup/restore functionality
- Safety configuration verification

**Safety Level**: ✅ Safe (no external API calls)

### 2. OAuth Integration Tests (`oauth_integration_test.rs`)

**Purpose**: Test real Google OAuth 2.0 authentication flow.

**Tests**:
- Authorization URL generation
- Token exchange process
- Token refresh mechanism
- Token expiry detection
- Complete OAuth flow simulation
- Error handling scenarios

**Safety Level**: ⚠️ Requires setup (real OAuth credentials)

### 3. Google Drive API Tests (`google_drive_integration_test.rs`)

**Purpose**: Test real Google Drive API operations.

**Tests**:
- Folder creation and management
- File upload and download
- File listing and search
- File deletion
- Error handling
- Rate limiting behavior

**Safety Level**: ⚠️ Requires setup (real Drive API access)

### 4. Sync Workflow Tests (`sync_workflow_integration_test.rs`)

**Purpose**: Test complete synchronization workflows.

**Tests**:
- End-to-end sync process
- Incremental sync scenarios
- Conflict resolution
- Compression type testing
- Error recovery mechanisms

**Safety Level**: ⚠️ Requires setup (full integration testing)

### 5. Safety Tests (`test_safety.rs`)

**Purpose**: Test safety mechanisms and cleanup procedures.

**Tests**:
- Backup/restore functionality
- Cleanup registry operations
- Emergency procedures
- Safety checkpoint creation
- Environment verification

**Safety Level**: ✅ Safe (internal safety mechanisms)

## Safety Mechanisms

### 1. Configuration Safety

```rust
// Example: Safe namespace generation
pub fn get_test_id(&self) -> String {
    format!("{}{}", 
        self.safety.namespace_prefix,  // e.g., "test_20241029_"
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    )
}
```

### 2. Backup Management

```rust
// Example: User configuration backup
pub fn backup_user_configs(&self) -> Result<Vec<PathBuf>> {
    let config_paths = vec![
        home_dir.join(".agentic-warden/auth.json"),
        home_dir.join(".agentic-warden/sync.json"),
    ];
    
    for config_path in config_paths {
        if config_path.exists() {
            let backup_path = config_path.with_extension("json.backup");
            fs::copy(&config_path, &backup_path)?;
        }
    }
}
```

### 3. Cleanup Registry

```rust
// Example: Resource tracking
pub fn register_cleanup_task(&self, 
    resource_id: String,
    resource_type: ResourceType,
    cleanup_action: CleanupAction,
    priority: CleanupPriority,
) -> Result<String> {
    let task = CleanupTask {
        id: generate_task_id(),
        resource_id,
        resource_type,
        cleanup_action,
        priority,
        timestamp: SystemTime::now(),
    };
    
    self.cleanup_registry.lock().unwrap().push(task);
}
```

### 4. Emergency Procedures

```rust
// Example: Emergency cleanup
pub async fn emergency_cleanup(&self) -> Result<()> {
    // Set emergency stop flag
    *self.emergency_stop.lock().unwrap() = true;
    
    // Perform cleanup in priority order
    let tasks = self.get_cleanup_tasks_sorted_by_priority();
    
    for task in tasks {
        self.perform_cleanup_task(&task).await?;
    }
    
    // Restore user configurations
    self.restore_user_configurations()?;
}
```

## CI/CD Integration

### GitHub Actions Workflow

The project includes a comprehensive GitHub Actions workflow (`integration-tests.yml`) with:

- **Conditional Execution**: Runs based on branch, schedule, or manual trigger
- **Parallel Execution**: Multiple test suites run in parallel
- **Dependency Management**: Proper job dependencies and failure handling
- **Artifact Management**: Upload and download of test results
- **Security**: Environment-based secrets management
- **Cleanup**: Automatic cleanup of test artifacts

### Environment Protection

- **Branch Protection**: Integration tests only run on protected branches
- **Secret Management**: OAuth credentials stored as GitHub secrets
- **Approval Required**: Manual approval for production-like testing
- **Rate Limiting**: Configurable delays to avoid API quota issues

### Test Matrix

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
    test_type: [mock, oauth, drive, sync]
    dry_run: [true, false]
```

## Troubleshooting

### Common Issues

#### 1. Authentication Failures

**Problem**: OAuth authentication fails with "invalid_client" or similar errors.

**Solution**:
```bash
# Verify client ID and secret
echo $AGENTIC_WARDEN_TEST_CLIENT_ID
echo $AGENTIC_WARDEN_TEST_CLIENT_SECRET

# Check redirect URI configuration
# Must be exactly: urn:ietf:wg:oauth:2.0:oob

# Re-run authentication flow
cargo test --test oauth_integration_test test_real_oauth_complete_flow_simulation -- --ignored
```

#### 2. API Quota Issues

**Problem**: Google Drive API returns rate limit errors.

**Solution**:
```bash
# Increase rate limiting delays
export AGENTIC_WARDEN_TEST_RATE_LIMIT_DELAY_MS=5000

# Run tests sequentially
cargo test --test google_drive_integration_test -- --test-threads=1

# Use dry-run mode for testing
export AGENTIC_WARDEN_TEST_DRY_RUN=true
```

#### 3. File Permission Issues

**Problem**: Tests fail with permission denied errors.

**Solution**:
```bash
# Check file permissions
ls -la ~/.agentic-warden/

# Reset permissions
chmod 600 ~/.agentic-warden/*.json
chmod 700 ~/.agentic-warden/

# Use fresh temporary directory
export TMPDIR=/tmp/test_${USER}
mkdir -p $TMPDIR
```

#### 4. Network Issues

**Problem**: Tests fail with network connectivity errors.

**Solution**:
```bash
# Check network connectivity
curl -I https://www.googleapis.com

# Test with dry-run mode
export AGENTIC_WARDEN_TEST_DRY_RUN=true

# Increase timeouts
export AGENTIC_WARDEN_TEST_AUTH_TIMEOUT_MS=600000  # 10 minutes
```

### Debug Mode

Enable comprehensive debugging:

```bash
export RUST_LOG=debug
export AGENTIC_WARDEN_TEST_VERBOSE=true
export AGENTIC_WARDEN_TEST_LOG_LEVEL=debug

# Run with specific test filter
cargo test --test integration_test_name -- --exact --nocapture
```

### Emergency Recovery

If tests leave artifacts or corrupt configurations:

```bash
# Emergency cleanup script
./scripts/emergency_cleanup.sh

# Manual configuration restore
cp ~/.agentic-warden/*.json.backup ~/.agentic-warden/

# Remove test artifacts
rm -rf /tmp/agentic-warden-test-*
```

## Best Practices

### 1. Development Workflow

1. **Start with Mock Tests**: Always run mock tests first
2. **Use Dry-Run Mode**: Test flows without real API calls
3. **Incremental Testing**: Test one component at a time
4. **Verify Safety**: Check safety mechanisms before real tests
5. **Monitor Resources**: Watch for resource leaks

### 2. Test Design

1. **Isolation**: Each test should be independent
2. **Cleanup**: Always clean up created resources
3. **Idempotency**: Tests should be runnable multiple times
4. **Error Coverage**: Test both success and failure scenarios
5. **Documentation**: Document test purpose and requirements

### 3. CI/CD Best Practices

1. **Parallel Execution**: Run independent tests in parallel
2. **Resource Limits**: Set reasonable timeouts and limits
3. **Artifact Management**: Clean up test artifacts automatically
4. **Security**: Never commit credentials to repository
5. **Monitoring**: Monitor test execution and failures

### 4. Security Practices

1. **Namespace Isolation**: Use unique test namespaces
2. **Credential Rotation**: Rotate test credentials regularly
3. **Access Control**: Limit test account permissions
4. **Audit Trail**: Log all test activities
5. **Emergency Procedures**: Have cleanup procedures ready

### 5. Performance Considerations

1. **Rate Limiting**: Respect API rate limits
2. **Batch Operations**: Use batch operations where possible
3. **Caching**: Cache expensive operations
4. **Resource Cleanup**: Clean up resources promptly
5. **Monitoring**: Monitor test performance metrics

## Contributing

When contributing to integration tests:

1. **Test Safety**: Ensure tests don't impact user data
2. **Documentation**: Document test requirements and setup
3. **Error Handling**: Include comprehensive error handling
4. **Cleanup**: Implement proper cleanup procedures
5. **CI/CD**: Update CI/CD workflows as needed

### Adding New Tests

1. Create test file in `tests/` directory
2. Implement safety mechanisms
3. Add to CI/CD workflow
4. Update documentation
5. Test locally before submitting

### Code Review Checklist

- [ ] Safety mechanisms implemented
- [ ] Cleanup procedures in place
- [ ] Error handling comprehensive
- [ ] Documentation updated
- [ ] CI/CD workflow updated
- [ ] Local testing completed

---

For more information or issues, please refer to the project repository or contact the development team.