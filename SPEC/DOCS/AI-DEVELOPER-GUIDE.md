# AI Developer Guide - Transparent Parameter Forwarding Implementation

## Project Overview

AIW (Agentic Warden) is an intelligent platform for managing AI CLI tools (Claude, Codex, Gemini) with provider management, process tracking, and transparent parameter forwarding.

## Current Version and Task

**Version**: v0.5.61
**Task**: Implement transparent parameter forwarding functionality for REQ-008 and REQ-009

## Project Structure

```
/home/putao/code/rust/aiw/
├── src/
│   ├── main.rs                 # Entry point and handle_external_ai_cli function
│   ├── commands/
│   │   ├── parser.rs           # Argument parsing logic
│   │   └── ai_cli.rs           # AiCliCommand execution logic
│   ├── cli_type.rs             # CLI-specific argument building
│   ├── supervisor.rs           # Process execution and environment injection
│   ├── provider/               # Provider management module (REUSE)
│   └── ...                     # Other modules
├── SPEC/
│   ├── 01-REQUIREMENTS.md      # REQ-008, REQ-009 (transparent parameter forwarding)
│   ├── 02-ARCHITECTURE.md      # ARCH-002, ARCH-008 (parameter handling)
│   └── DOCS/                   # This directory
├── tests/
│   ├── integration/            # Integration tests
│   └── unit/                   # Unit tests (add parameter_forwarding.rs)
└── Cargo.toml
```

## Current Implementation Analysis

### Existing Parameter Processing Logic

**Current Flow**:
1. `main.rs:handle_external_ai_cli` (lines 66-127) parses command line arguments
2. Only recognizes `-p`/`--provider` flags
3. All other arguments (including `-` prefixed) are treated as prompt content
4. Hard-coded arguments are passed to AI CLI via `cli_type.rs` methods

**Problem**: Cannot pass AI CLI-specific parameters (like `--model`, `--debug`, etc.)

### Reusable Components

**✅ Directly Reuse (No Changes Needed)**:
- Environment variable injection (`supervisor.rs` lines 265-268)
- Provider management logic (`provider/` module)
- Process execution framework (`supervisor.rs`)
- Error handling and logging infrastructure

**🟡 Needs Refactoring (Structure Reuse, Logic Rewrite)**:
- `handle_external_ai_cli` function framework (keep structure, rewrite parsing)
- `AiCliCommand` structure (extend with new fields)

**❌ Destroy and Rebuild (Complete Rewrite)**:
- `build_full_access_args` and `build_interactive_args` methods in `cli_type.rs`
- Simple parameter collection logic that treats everything as prompt

## Implementation Strategy

### Phase 1: Parameter Separation Logic

**New Data Structure**:
```rust
#[derive(Debug, Clone)]
pub struct SeparatedArgs {
    pub provider: Option<String>,
    pub cli_args: Vec<String>,    // Forwarded CLI parameters
    pub prompt: String,
}
```

**Parameter Separation Rules**:
1. Process `-p`/`--provider` first (consumed by AIW)
2. Collect other `-` prefixed parameters (forwarded to AI CLI)
3. Remaining arguments form the prompt
4. Validate parameter order (provider flags must come first)

### Phase 2: Enhanced CLI Argument Building

**Replace Hard-coded Methods**:
```rust
// NEW: Support forwarded CLI parameters
impl CliType {
    pub fn build_full_access_args_with_cli(&self, prompt: &str, cli_args: &[String]) -> Vec<String>
    pub fn build_interactive_args_with_cli(&self, cli_args: &[String]) -> Vec<String>
}
```

### Phase 3: Execution Path Integration

**Update Function Signatures**:
```rust
// Update supervisor functions to accept forwarded parameters
pub async fn execute_cli(registry: &dyn Registry, cli_type: &CliType,
                        args: &[OsString], provider: Option<String>,
                        cli_args: &[String]) -> Result<i32>
```

## Key Implementation Files

### 1. `/home/putao/code/rust/aiw/src/main.rs`
**Location**: `handle_external_ai_cli` function (lines 66-127)
**Changes**: Replace parameter parsing logic with separation approach
**Reuse**: Process startup and error handling framework

### 2. `/home/putao/code/rust/aiw/src/cli_type.rs`
**Location**: `build_full_access_args` and `build_interactive_args` methods
**Changes**: Complete rewrite to support forwarded parameters
**Reuse**: CLI type enum and basic display logic

### 3. `/home/putao/code/rust/aiw/src/commands/ai_cli.rs`
**Location**: `AiCliCommand` struct and `execute` method
**Changes**: Add `cli_args` field and pass through execution path
**Reuse**: Process management and error handling

### 4. `/home/putao/code/rust/aiw/src/commands/parser.rs`
**Location**: Add new parameter separation functions
**Changes**: New implementation for argument parsing
**Reuse**: Error handling patterns and validation logic

### 5. `/home/putao/code/rust/aiw/src/supervisor.rs`
**Location**: `execute_cli` and `start_interactive_cli` functions
**Changes**: Update function signatures to accept `cli_args`
**Reuse**: Environment injection, process execution, signal handling

## Usage Examples (From SPEC)

**Task Mode Examples**:
```bash
# Provider selection with transparent parameter forwarding
aiw claude -p glm --model sonnet --debug api "explain this code"
# Provider: glm, Forwarded: --model sonnet --debug api, Prompt: "explain this code"

# Structured output
aiw claude -p glm --print --output-format json "get structured response"

# Multiple parameters
aiw claude -p glm --model sonnet --max-budget-usd 5 --dangerously-skip-permissions "help me debug"
```

**Interactive Mode Examples**:
```bash
# Interactive with custom model and debugging
aiw claude -p glm --model sonnet --debug api
# Provider: glm, Forwarded: --model sonnet --debug api, No prompt (interactive)

# Interactive with output formatting
aiw claude -p glm --print --output-format stream-json --allowed-tools Bash,Edit
```

## Error Handling Requirements

**Parameter Order Validation**:
```rust
// Provider flags must come before other CLI parameters
// ❌ Error: aiw claude --model sonnet -p glm "prompt"
// ✅ Correct: aiw claude -p glm --model sonnet "prompt"
```

**User-Friendly Error Messages**:
```rust
"Error: -p/--provider must be specified before other CLI parameters.
Usage: agentic-warden claude -p provider --cli-param 'prompt'"
```

## Testing Strategy

**Unit Tests** (`tests/unit/parameter_forwarding.rs`):
- Parameter separation logic
- Order validation
- Error conditions

**Integration Tests** (`tests/integration/`):
- Complete workflows with forwarded parameters
- Backward compatibility
- Both task and interactive modes

## SPEC References

- **REQ-008**: 指定供应商模式AI CLI启动 (Transparent parameter forwarding)
- **REQ-009**: 交互式AI CLI启动 (Interactive mode parameter forwarding)
- **ARCH-002**: 供应商管理架构
- **ARCH-008**: 命令行参数处理架构

## Development Environment

**Rust Version**: 1.70+ (as specified in Cargo.toml)
**Key Dependencies**: clap, tokio, serde
**Test Framework**: Built-in Rust testing with pytest for integration

## Implementation Priority

1. **Critical**: Parameter separation logic (Phase 1)
2. **High**: CLI argument building (Phase 2)
3. **High**: Execution path integration (Phase 3)
4. **Medium**: Error handling and validation
5. **Required**: Comprehensive testing

## Related GitHub Issue

#21: feat: 实现透明参数转发功能 v0.5.23 [REQ-008, REQ-009]

---

This guide provides the essential context for implementing transparent parameter forwarding while maintaining consistency with the existing codebase architecture and SPEC requirements.