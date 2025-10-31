#!/bin/bash

# Emergency Cleanup Script for Agentic-Warden Integration Tests
#
# This script provides emergency cleanup procedures for integration tests
# that may have failed or left artifacts behind.

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LOG_FILE="/tmp/agentic-warden-emergency-cleanup-$(date +%Y%m%d_%H%M%S).log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] $*" | tee -a "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}$(date '+%Y-%m-%d %H:%M:%S') [WARN] $*${NC}" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}$(date '+%Y-%m-%d %H:%M:%S') [ERROR] $*${NC}" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}$(date '+%Y-%m-%d %H:%M:%S') [SUCCESS] $*${NC}" | tee -a "$LOG_FILE"
}

info() {
    echo -e "${BLUE}$(date '+%Y-%m-%d %H:%M:%S') [INFO] $*${NC}" | tee -a "$LOG_FILE"
}

# Emergency cleanup functions
cleanup_temp_directories() {
    log "Cleaning up temporary directories..."

    # Clean up test temporary directories
    local temp_dirs=(
        "/tmp/agentic-warden-test-*"
        "/tmp/test_*"
        "/tmp/cargo-test-*"
        "$HOME/.cargo/target/tmp"
    )

    for pattern in "${temp_dirs[@]}"; do
        for dir in $pattern; do
            if [[ -d "$dir" ]] && [[ -n "$(ls -A "$dir" 2>/dev/null)" ]]; then
                warn "Removing temporary directory: $dir"
                rm -rf "$dir" 2>/dev/null || error "Failed to remove $dir"
            fi
        done
    done

    success "Temporary directories cleaned up"
}

restore_user_configurations() {
    log "Restoring user configurations from backups..."

    local config_dir="$HOME/.agentic-warden"
    local backup_count=0

    if [[ -d "$config_dir" ]]; then
        # Find and restore backup files
        find "$config_dir" -name "*.backup" -type f | while read -r backup_file; do
            local original_file="${backup_file%.backup}"

            if [[ ! -f "$original_file" ]] || [[ "$backup_file" -nt "$original_file" ]]; then
                warn "Restoring configuration: $original_file"
                cp "$backup_file" "$original_file" || error "Failed to restore $original_file"
                ((backup_count++))

                # Remove backup after successful restore
                rm "$backup_file" || warn "Failed to remove backup: $backup_file"
            fi
        done
    fi

    success "User configurations restored ($backup_count files)"
}

cleanup_test_artifacts() {
    log "Cleaning up test artifacts..."

    # Clean up cargo test artifacts
    if [[ -d "$PROJECT_ROOT/target" ]]; then
        find "$PROJECT_ROOT/target" -name "test-*" -type d -exec rm -rf {} + 2>/dev/null || true
        find "$PROJECT_ROOT/target" -name "*.json" -path "*/test-*" -delete 2>/dev/null || true
    fi

    # Clean up cache files
    local cache_dirs=(
        "$HOME/.cache/agentic-warden"
        "$HOME/.cache/cargo/test"
    )

    for cache_dir in "${cache_dirs[@]}"; do
        if [[ -d "$cache_dir" ]]; then
            warn "Cleaning cache directory: $cache_dir"
            find "$cache_dir" -name "*test*" -delete 2>/dev/null || true
        fi
    done

    success "Test artifacts cleaned up"
}

cleanup_google_drive_artifacts() {
    log "Note: Google Drive artifacts require manual cleanup"
    warn "Please check your Google Drive for test folders with names starting with 'test_'"
    warn "Test folders to look for:"
    warn "  - test_* (integration test folders)"
    warn "  - agentic-warden-integration-tests"
    warn "  - Any folders created during test runs"

    read -p "Do you want to open Google Drive to check for test artifacts? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if command -v xdg-open > /dev/null; then
            xdg-open "https://drive.google.com" 2>/dev/null || warn "Could not open Google Drive"
        elif command -v open > /dev/null; then
            open "https://drive.google.com" 2>/dev/null || warn "Could not open Google Drive"
        else
            info "Please manually visit: https://drive.google.com"
        fi
    fi
}

cleanup_processes() {
    log "Cleaning up test processes..."

    # Find and kill any lingering test processes
    local test_processes=(
        "agentic-warden"
        "cargo test"
        "integration_test"
    )

    for proc_name in "${test_processes[@]}"; do
        local pids=$(pgrep -f "$proc_name" 2>/dev/null || true)
        if [[ -n "$pids" ]]; then
            warn "Killing test processes: $proc_name (PIDs: $pids)"
            echo "$pids" | xargs kill -TERM 2>/dev/null || true
            sleep 2

            # Force kill if still running
            local remaining_pids=$(pgrep -f "$proc_name" 2>/dev/null || true)
            if [[ -n "$remaining_pids" ]]; then
                warn "Force killing remaining processes: $proc_name (PIDs: $remaining_pids)"
                echo "$remaining_pids" | xargs kill -KILL 2>/dev/null || true
            fi
        fi
    done

    success "Test processes cleaned up"
}

cleanup_environment_variables() {
    log "Resetting environment variables..."

    # Test-related environment variables to unset
    local test_env_vars=(
        "AGENTIC_WARDEN_TEST_CLIENT_ID"
        "AGENTIC_WARDEN_TEST_CLIENT_SECRET"
        "AGENTIC_WARDEN_TEST_REDIRECT_URI"
        "AGENTIC_WARDEN_TEST_NAMESPACE_PREFIX"
        "AGENTIC_WARDEN_TEST_BASE_FOLDER"
        "AGENTIC_WARDEN_TEST_LOG_LEVEL"
        "AGENTIC_WARDEN_TEST_VERBOSE"
        "AGENTIC_WARDEN_TEST_DRY_RUN"
        "AGENTIC_WARDEN_TEST_MAX_FILES"
        "AGENTIC_WARDEN_TEST_CLEANUP_ON_SUCCESS"
        "AGENTIC_WARDEN_TEST_CLEANUP_ON_FAILURE"
        "AGENTIC_WARDEN_TEST_BACKUP_CONFIG"
        "AGENTIC_WARDEN_TEST_RESTORE_CONFIG"
        "AGENTIC_WARDEN_RUN_INTEGRATION_TESTS"
        "AGENTIC_WARDEN_TEST_RATE_LIMIT_DELAY_MS"
        "AGENTIC_WARDEN_TEST_MAX_FILE_SIZE"
        "AGENTIC_WARDEN_TEST_AUTH_TIMEOUT_MS"
        "AGENTIC_WARDEN_TEST_HEADLESS_AUTH"
    )

    local unset_count=0
    for var_name in "${test_env_vars[@]}"; do
        if [[ -n "${!var_name:-}" ]]; then
            unset "$var_name" || warn "Failed to unset $var_name"
            ((unset_count++))
        fi
    done

    success "Environment variables reset ($unset_count variables)"
}

verify_cleanup() {
    log "Verifying cleanup completeness..."

    local issues=0

    # Check for remaining test processes
    local remaining_processes=$(pgrep -f "agentic-warden\|integration_test" 2>/dev/null || true)
    if [[ -n "$remaining_processes" ]]; then
        error "Remaining test processes found: $remaining_processes"
        ((issues++))
    fi

    # Check for remaining temporary directories
    local remaining_temp=$(find /tmp -name "test_*" -type d 2>/dev/null | wc -l)
    if [[ "$remaining_temp" -gt 0 ]]; then
        warn "Remaining temporary directories: $remaining_temp"
        ((issues++))
    fi

    # Check for backup files
    local backup_files=$(find "$HOME/.agentic-warden" -name "*.backup" 2>/dev/null | wc -l)
    if [[ "$backup_files" -gt 0 ]]; then
        warn "Remaining backup files: $backup_files"
        ((issues++))
    fi

    if [[ "$issues" -eq 0 ]]; then
        success "Cleanup verification passed - no issues found"
    else
        warn "Cleanup verification completed with $issues issues"
    fi
}

generate_cleanup_report() {
    log "Generating cleanup report..."

    cat << EOF > "/tmp/agentic-warden-cleanup-report-$(date +%Y%m%d_%H%M%S).txt"
===========================================
Agentic-Warden Emergency Cleanup Report
===========================================

Timestamp: $(date)
Script: $0
Log File: $LOG_FILE

Cleanup Actions Performed:
- Temporary directories cleaned
- User configurations restored from backups
- Test artifacts removed
- Test processes terminated
- Environment variables reset
- Cleanup verification performed

System Status:
- Home Directory: $HOME
- Project Root: $PROJECT_ROOT
- Current User: $(whoami)
- Shell: $SHELL

Remaining Issues:
$(find /tmp -name "test_*" -type d 2>/dev/null || echo "None found")

Recommendations:
1. Check Google Drive manually for test artifacts
2. Monitor system for unusual activity
3. Review log file for details: $LOG_FILE
4. Restart any services if needed

EOF

    success "Cleanup report generated"
}

# Main cleanup workflow
main() {
    info "Starting emergency cleanup for agentic-warden integration tests..."
    info "Log file: $LOG_FILE"

    # Trap for cleanup on script exit
    trap 'error "Emergency cleanup script interrupted"; exit 1' INT TERM

    # Perform cleanup steps
    cleanup_processes
    cleanup_temp_directories
    restore_user_configurations
    cleanup_test_artifacts
    cleanup_environment_variables
    cleanup_google_drive_artifacts

    # Verify and report
    verify_cleanup
    generate_cleanup_report

    success "Emergency cleanup completed successfully!"
    info "Check the log file for details: $LOG_FILE"
    info "Manual steps may be required for Google Drive cleanup"
}

# Interactive mode
interactive_mode() {
    echo "=== Agentic-Warden Emergency Cleanup ==="
    echo "This script will clean up artifacts from integration tests"
    echo
    echo "Actions to be performed:"
    echo "1. Clean up temporary directories"
    echo "2. Restore user configurations from backups"
    echo "3. Remove test artifacts"
    echo "4. Terminate test processes"
    echo "5. Reset environment variables"
    echo "6. Guide Google Drive cleanup"
    echo

    read -p "Do you want to proceed with emergency cleanup? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        main
    else
        info "Emergency cleanup cancelled"
        exit 0
    fi
}

# Command line options
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  -h, --help       Show this help message"
    echo "  -i, --interactive Run in interactive mode"
    echo "  -a, --all        Run all cleanup steps"
    echo "  -p, --processes  Clean up test processes only"
    echo "  -t, --temp       Clean up temporary directories only"
    echo "  -c, --config     Restore user configurations only"
    echo "  -f, --artifacts  Clean up test artifacts only"
    echo "  -e, --env        Reset environment variables only"
    echo "  -v, --verify     Verify cleanup completeness only"
    echo
    echo "Examples:"
    echo "  $0 -i            # Interactive mode"
    echo "  $0 -a            # Run all cleanup steps"
    echo "  $0 -p -t         # Clean processes and temp dirs"
}

# Parse command line arguments
case "${1:-}" in
    -h|--help)
        usage
        exit 0
        ;;
    -i|--interactive)
        interactive_mode
        ;;
    -a|--all)
        main
        ;;
    -p|--processes)
        cleanup_processes
        ;;
    -t|--temp)
        cleanup_temp_directories
        ;;
    -c|--config)
        restore_user_configurations
        ;;
    -f|--artifacts)
        cleanup_test_artifacts
        ;;
    -e|--env)
        cleanup_environment_variables
        ;;
    -v|--verify)
        verify_cleanup
        ;;
    "")
        interactive_mode
        ;;
    *)
        echo "Unknown option: $1"
        usage
        exit 1
        ;;
esac
