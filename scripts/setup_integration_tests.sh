#!/bin/bash

# Setup Script for Agentic-Warden Integration Tests
#
# This script helps set up the environment for running integration tests,
# including OAuth credentials and configuration verification.

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ENV_FILE="$PROJECT_ROOT/.env"
ENV_EXAMPLE="$PROJECT_ROOT/.env.example"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] $*"
}

warn() {
    echo -e "${YELLOW}$(date '+%Y-%m-%d %H:%M:%S') [WARN] $*${NC}"
}

error() {
    echo -e "${RED}$(date '+%Y-%m-%d %H:%M:%S') [ERROR] $*${NC}"
}

success() {
    echo -e "${GREEN}$(date '+%Y-%m-%d %H:%M:%S') [SUCCESS] $*${NC}"
}

info() {
    echo -e "${BLUE}$(date '+%Y-%m-%d %H:%M:%S') [INFO] $*${NC}"
}

# Function to prompt for input
prompt() {
    local prompt="$1"
    local variable="$2"
    local default="${3:-}"
    local sensitive="${4:-false}"

    while true; do
        if [[ "$sensitive" == "true" ]]; then
            read -s -p "$prompt [$default]: " input
            echo
        else
            read -p "$prompt [$default]: " input
        fi

        if [[ -z "$input" && -n "$default" ]]; then
            input="$default"
        fi

        if [[ -n "$input" ]]; then
            export "$variable"="$input"
            break
        else
            warn "Input cannot be empty. Please try again."
        fi
    done
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check system requirements
check_system_requirements() {
    info "Checking system requirements..."

    # Check Rust installation
    if ! command_exists rustc; then
        error "Rust is not installed. Please install Rust from https://rustup.rs/"
        exit 1
    fi

    local rust_version=$(rustc --version)
    info "Rust version: $rust_version"

    # Check Cargo
    if ! command_exists cargo; then
        error "Cargo is not installed. Please install Rust with Cargo included."
        exit 1
    fi

    # Check OpenSSL (Linux)
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if ! pkg-config --exists openssl; then
            warn "OpenSSL development libraries may be required."
            warn "On Ubuntu/Debian: sudo apt-get install pkg-config libssl-dev"
            warn "On Fedora: sudo dnf install pkg-config openssl-devel"
        fi
    fi

    # Check network connectivity
    if ! curl -s --connect-timeout 5 https://www.googleapis.com >/dev/null; then
        warn "Cannot connect to Google APIs. Check your internet connection."
    else
        success "Network connectivity verified"
    fi

    success "System requirements check completed"
}

# Function to create example .env file
create_env_example() {
    cat > "$ENV_EXAMPLE" << 'EOF'
# Agentic-Warden Integration Test Configuration
# Copy this file to .env and fill in your credentials

# Enable integration tests
AGENTIC_WARDEN_RUN_INTEGRATION_TESTS=true

# Google OAuth Test Credentials
# Get these from Google Cloud Console
AGENTIC_WARDEN_TEST_CLIENT_ID=your_google_oauth_client_id_here
AGENTIC_WARDEN_TEST_CLIENT_SECRET=your_google_oauth_client_secret_here
AGENTIC_WARDEN_TEST_REDIRECT_URI=urn:ietf:wg:oauth:2.0:oob

# Test Configuration
AGENTIC_WARDEN_TEST_NAMESPACE_PREFIX=test_local_
AGENTIC_WARDEN_TEST_BASE_FOLDER=agentic-warden-integration-tests
AGENTIC_WARDEN_TEST_LOG_LEVEL=info
AGENTIC_WARDEN_TEST_VERBOSE=true
AGENTIC_WARDEN_TEST_DRY_RUN=false

# Safety Settings
AGENTIC_WARDEN_TEST_MAX_FILES_TOTAL=100
AGENTIC_WARDEN_TEST_CLEANUP_ON_SUCCESS=true
AGENTIC_WARDEN_TEST_CLEANUP_ON_FAILURE=true
AGENTIC_WARDEN_TEST_BACKUP_CONFIG=true
AGENTIC_WARDEN_TEST_RESTORE_CONFIG=true
AGENTIC_WARDEN_TEST_REQUIRE_MANUAL_APPROVAL=false

# API Configuration
AGENTIC_WARDEN_TEST_RATE_LIMIT_DELAY_MS=2000
AGENTIC_WARDEN_TEST_MAX_FILE_SIZE=10485760  # 10MB
AGENTIC_WARDEN_TEST_MAX_FILES_PER_TEST=10

# Authentication Configuration
AGENTIC_WARDEN_TEST_AUTH_TIMEOUT_MS=300000  # 5 minutes
AGENTIC_WARDEN_TEST_HEADLESS_AUTH=false

# Environment Configuration
AGENTIC_WARDEN_TEST_LOG_LEVEL=info
AGENTIC_WARDEN_TEST_VERBOSE=false
AGENTIC_WARDEN_TEST_DRY_RUN=false

# Compression Settings
AGENTIC_WARDEN_TEST_MAX_FILE_SIZE=10485760  # 10MB
EOF

    success "Created example .env file: $ENV_EXAMPLE"
}

# Function to setup OAuth credentials
setup_oauth_credentials() {
    info "Setting up Google OAuth credentials..."

    echo
    info "To get Google OAuth credentials:"
    echo "1. Go to Google Cloud Console: https://console.cloud.google.com/"
    echo "2. Create a new project or select an existing one"
    echo "3. Enable Google Drive API"
    echo "4. Go to 'APIs & Services' > 'Credentials'"
    echo "5. Click 'Create Credentials' > 'OAuth client ID'"
    echo "6. Select 'Desktop application'"
    echo "7. Set authorized redirect URIs to: urn:ietf:wg:oauth:2.0:oob"
    echo "8. Copy the Client ID and Client Secret"
    echo

    prompt "Enter your Google OAuth Client ID" "CLIENT_ID"
    prompt "Enter your Google OAuth Client Secret" "CLIENT_SECRET" "" true

    echo
    info "Verifying OAuth credentials..."

    # Basic validation
    if [[ ${#CLIENT_ID} -lt 10 ]]; then
        error "Client ID appears to be invalid (too short)"
        exit 1
    fi

    if [[ ${#CLIENT_SECRET} -lt 10 ]]; then
        error "Client Secret appears to be invalid (too short)"
        exit 1
    fi

    success "OAuth credentials validated"
}

# Function to create .env file
create_env_file() {
    info "Creating .env file..."

    local namespace="test_local_$(date +%Y%m%d_%H%M%S)_"

    cat > "$ENV_FILE" << EOF
# Agentic-Warden Integration Test Configuration
# Generated on $(date)

# Enable integration tests
AGENTIC_WARDEN_RUN_INTEGRATION_TESTS=true

# Google OAuth Test Credentials
AGENTIC_WARDEN_TEST_CLIENT_ID=$CLIENT_ID
AGENTIC_WARDEN_TEST_CLIENT_SECRET=$CLIENT_SECRET
AGENTIC_WARDEN_TEST_REDIRECT_URI=urn:ietf:wg:oauth:2.0:oob

# Test Configuration
AGENTIC_WARDEN_TEST_NAMESPACE_PREFIX=$namespace
AGENTIC_WARDEN_TEST_BASE_FOLDER=agentic-warden-integration-tests
AGENTIC_WARDEN_TEST_LOG_LEVEL=info
AGENTIC_WARDEN_TEST_VERBOSE=true
AGENTIC_WARDEN_TEST_DRY_RUN=false

# Safety Settings
AGENTIC_WARDEN_TEST_MAX_FILES_TOTAL=100
AGENTIC_WARDEN_TEST_CLEANUP_ON_SUCCESS=true
AGENTIC_WARDEN_TEST_CLEANUP_ON_FAILURE=true
AGENTIC_WARDEN_TEST_BACKUP_CONFIG=true
AGENTIC_WARDEN_TEST_RESTORE_CONFIG=true
AGENTIC_WARDEN_TEST_REQUIRE_MANUAL_APPROVAL=false

# API Configuration
AGENTIC_WARDEN_TEST_RATE_LIMIT_DELAY_MS=2000
AGENTIC_WARDEN_TEST_MAX_FILE_SIZE=10485760
AGENTIC_WARDEN_TEST_MAX_FILES_PER_TEST=10

# Authentication Configuration
AGENTIC_WARDEN_TEST_AUTH_TIMEOUT_MS=300000
AGENTIC_WARDEN_TEST_HEADLESS_AUTH=false

# Environment Configuration
AGENTIC_WARDEN_TEST_LOG_LEVEL=info
AGENTIC_WARDEN_TEST_VERBOSE=false
AGENTIC_WARDEN_TEST_DRY_RUN=false
EOF

    success "Created .env file: $ENV_FILE"
}

# Function to build the project
build_project() {
    info "Building the project..."

    cd "$PROJECT_ROOT"

    # Build the main project
    cargo build --release

    # Build test dependencies
    cargo test --no-run --features testing

    success "Project built successfully"
}

# Function to run configuration verification
verify_configuration() {
    info "Verifying configuration..."

    cd "$PROJECT_ROOT"

    # Load environment variables
    if [[ -f "$ENV_FILE" ]]; then
        source "$ENV_FILE"
        export $(cut -d= -f1 "$ENV_FILE")
    fi

    # Run configuration tests
    info "Running configuration verification tests..."
    cargo test --test integration_config -- --nocapture

    # Run OAuth configuration verification
    info "Running OAuth configuration verification..."
    cargo test --test oauth_integration_test test_oauth_configuration_verification -- --nocapture

    success "Configuration verification completed"
}

# Function to run initial authentication
initial_authentication() {
    info "Running initial OAuth authentication..."

    cd "$PROJECT_ROOT"

    # Load environment variables
    if [[ -f "$ENV_FILE" ]]; then
        source "$ENV_FILE"
        export $(cut -d= -f1 "$ENV_FILE")
    fi

    # Run OAuth flow simulation
    info "Running OAuth flow simulation (dry-run)..."
    export AGENTIC_WARDEN_TEST_DRY_RUN=true
    cargo test --test oauth_integration_test test_real_oauth_complete_flow_simulation -- --ignored --nocapture

    echo
    warn "Note: The above was a dry-run simulation."
    warn "For real authentication, run:"
    warn "  export AGENTIC_WARDEN_TEST_DRY_RUN=false"
    warn "  cargo test --test oauth_integration_test test_real_oauth_complete_flow_simulation -- --ignored --nocapture"

    success "Initial authentication setup completed"
}

# Function to provide next steps
show_next_steps() {
    echo
    info "=== Setup Complete! ==="
    echo
    info "Your integration test environment is now ready."
    echo
    echo "Next steps:"
    echo "1. Run mock tests (always safe):"
    echo "   cargo test --test oauth_mock_test"
    echo "   cargo test --test sync_isolated_test"
    echo
    echo "2. Run real integration tests:"
    echo "   cargo test --test oauth_integration_test -- --ignored"
    echo "   cargo test --test google_drive_integration_test -- --ignored"
    echo "   cargo test --test sync_workflow_integration_test -- --ignored"
    echo
    echo "3. Run tests with dry-run mode:"
    echo "   export AGENTIC_WARDEN_TEST_DRY_RUN=true"
    echo "   cargo test --test google_drive_integration_test -- --ignored"
    echo
    echo "4. For troubleshooting, run:"
    echo "   ./scripts/emergency_cleanup.sh"
    echo
    echo "5. For more information, see:"
    echo "   docs/integration-testing-guide.md"
    echo
    success "Happy testing! 🚀"
}

# Main setup workflow
main() {
    echo "=== Agentic-Warden Integration Test Setup ==="
    echo

    # Check system requirements
    check_system_requirements
    echo

    # Create example .env file
    create_env_example
    echo

    # Setup OAuth credentials
    setup_oauth_credentials
    echo

    # Create .env file
    create_env_file
    echo

    # Build project
    build_project
    echo

    # Verify configuration
    verify_configuration
    echo

    # Initial authentication
    initial_authentication
    echo

    # Show next steps
    show_next_steps
}

# Command line options
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  -h, --help       Show this help message"
    echo "  -c, --check      Check system requirements only"
    echo "  -b, --build      Build project only"
    echo "  -v, --verify     Verify configuration only"
    echo "  -a, --auth       Run initial authentication only"
    echo
    echo "Examples:"
    echo "  $0                # Run complete setup"
    echo "  $0 -c            # Check system requirements"
    echo "  $0 -b            # Build project only"
}

# Parse command line arguments
case "${1:-}" in
    -h|--help)
        usage
        exit 0
        ;;
    -c|--check)
        check_system_requirements
        ;;
    -b|--build)
        build_project
        ;;
    -v|--verify)
        verify_configuration
        ;;
    -a|--auth)
        initial_authentication
        ;;
    "")
        main
        ;;
    *)
        echo "Unknown option: $1"
        usage
        exit 1
        ;;
esac
