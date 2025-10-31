# Agentic-Warden Integration Testing Suite

A comprehensive real integration testing solution for the agentic-warden project using authentic Google OAuth and Drive API.

## 🚀 Quick Start

### Prerequisites
- Rust (latest stable)
- Google Cloud Project with Drive API enabled
- OAuth 2.0 credentials

### Setup
```bash
# Clone the repository
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden

# Run the setup script
./scripts/setup_integration_tests.sh

# Or set up manually:
# 1. Copy .env.example to .env
# 2. Fill in your Google OAuth credentials
# 3. Export environment variables
```

### Run Tests
```bash
# Mock tests (always safe)
cargo test --test oauth_mock_test
cargo test --test sync_isolated_test

# Real integration tests (requires setup)
cargo test --test oauth_integration_test -- --ignored
cargo test --test google_drive_integration_test -- --ignored
cargo test --test sync_workflow_integration_test -- --ignored

# Dry-run mode (no real API calls)
export AGENTIC_WARDEN_TEST_DRY_RUN=true
cargo test --test google_drive_integration_test -- --ignored
```

## 📋 Test Categories

### ✅ Safe Tests (No External Dependencies)
- **Mock OAuth Tests** (`oauth_mock_test.rs`): Complete OAuth flow simulation
- **Isolated Sync Tests** (`sync_isolated_test.rs`): Sync logic without external services
- **Safety Tests** (`test_safety.rs`): Safety mechanisms and cleanup procedures

### ⚠️ Real Integration Tests (Requires Setup)
- **OAuth Integration** (`oauth_integration_test.rs`): Real Google OAuth 2.0 flow
- **Google Drive API** (`google_drive_integration_test.rs`): Real Drive operations
- **Sync Workflows** (`sync_workflow_integration_test.rs`): End-to-end sync scenarios

## 🔒 Safety Features

### Multi-Layer Protection
1. **Namespace Isolation**: Unique test prefixes (`test_20241029_123456_`)
2. **Configuration Backup**: Automatic backup/restore of user configs
3. **Cleanup Registry**: Comprehensive resource tracking
4. **Emergency Procedures**: Immediate cleanup capability
5. **Dry-Run Mode**: Testing without real API calls

### Safety Configuration
```bash
# Safety settings
AGENTIC_WARDEN_TEST_CLEANUP_ON_SUCCESS=true
AGENTIC_WARDEN_TEST_CLEANUP_ON_FAILURE=true
AGENTIC_WARDEN_TEST_BACKUP_CONFIG=true
AGENTIC_WARDEN_TEST_RESTORE_CONFIG=true
```

### Emergency Cleanup
```bash
# Run emergency cleanup
./scripts/emergency_cleanup.sh

# Interactive mode
./scripts/emergency_cleanup.sh -i

# Clean specific components
./scripts/emergency_cleanup.sh -p  # processes only
./scripts/emergency_cleanup.sh -t  # temp files only
```

## 🏗️ Architecture

### Test Environment
```
Integration Test Environment
├── Configuration Layer
│   ├── Environment variables
│   ├── Test settings
│   └── Safety configuration
├── Safety Layer
│   ├── Backup management
│   ├── Cleanup registry
│   └── Emergency procedures
├── OAuth Layer
│   ├── Real authentication
│   ├── Token management
│   └── Credential storage
├── Google Drive Layer
│   ├── File operations
│   ├── Folder management
│   └── API error handling
└── Sync Workflow Layer
    ├── Configuration packing
    ├── Compression & upload
    ├── Download & extraction
    └── Conflict resolution
```

### Test Isolation
- **Unique Namespaces**: Each test run gets unique identifiers
- **Temporary Directories**: All test data in isolated temp directories
- **Rate Limiting**: Configurable delays between API calls
- **Resource Limits**: Maximum file sizes and counts

## 🔄 CI/CD Integration

### GitHub Actions Workflow
- **Conditional Execution**: Runs on schedule, manual trigger, or relevant changes
- **Parallel Testing**: Multiple test suites run concurrently
- **Safety First**: Dry-run mode for CI environments
- **Automatic Cleanup**: Clean up test artifacts after runs
- **Failure Handling**: Comprehensive error reporting and cleanup

### Environment Configuration
```yaml
# GitHub Actions Environment
environment: integration-testing
secrets:
  AGENTIC_WARDEN_TEST_CLIENT_ID: ${{ secrets.AGENTIC_WARDEN_TEST_CLIENT_ID }}
  AGENTIC_WARDEN_TEST_CLIENT_SECRET: ${{ secrets.AGENTIC_WARDEN_TEST_CLIENT_SECRET }}
```

## 📊 Test Coverage

### OAuth Integration Tests
- ✅ Authorization URL generation
- ✅ Token exchange process
- ✅ Token refresh mechanism
- ✅ Token expiry detection
- ✅ Complete OAuth flow simulation
- ✅ Error handling scenarios

### Google Drive API Tests
- ✅ Folder creation and management
- ✅ File upload and download
- ✅ File listing and search
- ✅ File deletion
- ✅ Error handling
- ✅ Rate limiting behavior

### Sync Workflow Tests
- ✅ End-to-end sync process
- ✅ Incremental sync scenarios
- ✅ Conflict resolution
- ✅ Compression type testing
- ✅ Error recovery mechanisms

### Safety Tests
- ✅ Backup/restore functionality
- ✅ Cleanup registry operations
- ✅ Emergency procedures
- ✅ Environment verification

## 🛠️ Configuration

### Environment Variables
```bash
# Enable integration tests
AGENTIC_WARDEN_RUN_INTEGRATION_TESTS=true

# OAuth credentials
AGENTIC_WARDEN_TEST_CLIENT_ID=your_client_id
AGENTIC_WARDEN_TEST_CLIENT_SECRET=your_client_secret

# Test configuration
AGENTIC_WARDEN_TEST_NAMESPACE_PREFIX=test_local_
AGENTIC_WARDEN_TEST_BASE_FOLDER=agentic-warden-integration-tests
AGENTIC_WARDEN_TEST_LOG_LEVEL=info
AGENTIC_WARDEN_TEST_VERBOSE=true

# Safety settings
AGENTIC_WARDEN_TEST_CLEANUP_ON_SUCCESS=true
AGENTIC_WARDEN_TEST_CLEANUP_ON_FAILURE=true
AGENTIC_WARDEN_TEST_BACKUP_CONFIG=true

# API settings
AGENTIC_WARDEN_TEST_RATE_LIMIT_DELAY_MS=2000
AGENTIC_WARDEN_TEST_MAX_FILE_SIZE=10485760  # 10MB
```

### Google Cloud Setup
1. **Create Project**: Google Cloud Console
2. **Enable APIs**: Google Drive API, OAuth 2.0 API
3. **Create Credentials**: OAuth 2.0 Client ID (Desktop application)
4. **Set Redirect URI**: `urn:ietf:wg:oauth:2.0:oob`
5. **Copy Credentials**: Client ID and Client Secret

## 🔧 Troubleshooting

### Common Issues

#### Authentication Failures
```bash
# Verify credentials
echo $AGENTIC_WARDEN_TEST_CLIENT_ID
echo $AGENTIC_WARDEN_TEST_CLIENT_SECRET

# Re-run authentication
cargo test --test oauth_integration_test test_real_oauth_complete_flow_simulation -- --ignored
```

#### API Quota Issues
```bash
# Increase rate limiting
export AGENTIC_WARDEN_TEST_RATE_LIMIT_DELAY_MS=5000

# Use dry-run mode
export AGENTIC_WARDEN_TEST_DRY_RUN=true
```

#### Permission Issues
```bash
# Check file permissions
ls -la ~/.agentic-warden/

# Fix permissions
chmod 600 ~/.agentic-warden/*.json
chmod 700 ~/.agentic-warden/
```

### Debug Mode
```bash
# Enable verbose debugging
export RUST_LOG=debug
export AGENTIC_WARDEN_TEST_VERBOSE=true

# Run with specific test filter
cargo test --test integration_test_name -- --exact --nocapture
```

## 📁 File Structure

```
agentic-warden/
├── tests/
│   ├── integration_config.rs          # Configuration management
│   ├── oauth_integration_test.rs      # Real OAuth tests
│   ├── google_drive_integration_test.rs # Real Drive API tests
│   ├── sync_workflow_integration_test.rs # End-to-end sync tests
│   ├── test_safety.rs                 # Safety mechanisms
│   ├── oauth_mock_test.rs            # Mock OAuth tests (existing)
│   └── sync_isolated_test.rs         # Isolated sync tests (existing)
├── scripts/
│   ├── setup_integration_tests.sh     # Setup script
│   └── emergency_cleanup.sh           # Emergency cleanup
├── docs/
│   └── integration-testing-guide.md   # Comprehensive guide
├── .github/
│   └── workflows/
│       └── integration-tests.yml      # CI/CD workflow
├── Cargo.toml                        # Test configuration
├── .env.example                      # Environment template
└── README_INTEGRATION_TESTS.md       # This file
```

## 🎯 Best Practices

### Development Workflow
1. **Start with Mock Tests**: Always run mock tests first
2. **Use Dry-Run Mode**: Test flows without real API calls
3. **Incremental Testing**: Test one component at a time
4. **Verify Safety**: Check safety mechanisms before real tests
5. **Monitor Resources**: Watch for resource leaks

### Test Design
- **Isolation**: Each test independent
- **Cleanup**: Always clean up created resources
- **Idempotency**: Tests runnable multiple times
- **Error Coverage**: Test both success and failure scenarios
- **Documentation**: Document test requirements

### Security Practices
- **Namespace Isolation**: Unique test namespaces
- **Credential Rotation**: Rotate test credentials regularly
- **Access Control**: Limit test account permissions
- **Audit Trail**: Log all test activities
- **Emergency Procedures**: Have cleanup procedures ready

## 📞 Support

### Documentation
- **Comprehensive Guide**: `docs/integration-testing-guide.md`
- **API Documentation**: Inline code documentation
- **Setup Instructions**: `scripts/setup_integration_tests.sh`

### Getting Help
- **Issues**: GitHub Issues for bug reports and feature requests
- **Discussions**: GitHub Discussions for questions and community support
- **Emergency**: Use emergency cleanup script if needed

## 🤝 Contributing

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

## 📈 Metrics

### Test Coverage
- **OAuth Tests**: 8 test scenarios
- **Drive API Tests**: 6 test scenarios
- **Sync Workflow Tests**: 4 test scenarios
- **Safety Tests**: 5 test scenarios
- **Mock Tests**: Complete coverage of existing functionality

### Performance
- **Rate Limiting**: Configurable delays (default 2 seconds)
- **File Size Limits**: 10MB default, configurable
- **Concurrent Testing**: Parallel execution where safe
- **Cleanup Efficiency**: Automatic resource cleanup

---

**Note**: This integration testing suite is designed for development and testing purposes. Always ensure you have proper authorization before running tests with real Google services.

For more detailed information, see the [comprehensive guide](docs/integration-testing-guide.md).