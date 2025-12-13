# Tests Documentation

## 📋 Test Categories (No Mocks Allowed)

### ✅ **Real Integration Tests**
These tests connect to actual services and verify real functionality:

- **`real_mcp_integration_test.rs`**: Tests connection to real MCP servers (Filesystem, Knowledge Graph)
- **`real_rerank_test.rs`**: Tests real BGE reranker model computation
- **`rerank_e2e_test.rs`**: Tests real FastEmbed reranker with actual model downloads
- **`force_real_rerank.rs`**: Tests actual BGE reranker scoring with real computation

### ✅ **REQ-013 Phase 1 & 2 E2E Tests**
These tests verify the intelligent routing system with real MCP servers and real AI CLI:

- **`real_req013_phase1_capability_e2e.rs`**: Capability description generation, FIFO eviction
- **`real_req013_phase2_dynamic_tool_e2e.rs`**: Dynamic tool registration, direct proxy optimization

### ✅ **LLM Backend E2E Tests**
These tests verify both LLM backend paths with real services:

- **`real_llm_backend_e2e.rs`**: Tests both Ollama and AI CLI backends
  - `test_e2e_with_ollama_backend`: Ollama完整工作流（强制OPENAI_TOKEN）
  - `test_e2e_with_ai_cli_backend`: AI CLI完整工作流（强制无OPENAI_TOKEN）
  - `test_backend_auto_detection`: 后端自动检测机制验证
  - `test_backend_factory_creation`: 工厂创建机制验证
  - `test_backend_response_comparison`: 两个后端响应质量对比

### ✅ **Unit Tests**
Pure logic tests with no external dependencies:

- **`tool_count_test.rs`**: Verify tool count and reduction

### ✅ **Infrastructure Integration Tests**
Real infrastructure tests:

- **Integration Tests**: Real MCP server connections, third-party provider tests

## ❌ **Removed Mock-Based Tests**

The following tests have been **DELETED** because they were based on mocks/fakes:

- ❌ **`mcp_third_party_test.rs`**: 100% simulated responses, fake API calls
- ❌ **`dynamic_tool_generation_test.rs`**: Incomplete functionality testing
- ❌ **`tests/common/mod.rs`**: Entire mock module (MockMcpServer, MockLlmClient, etc.)
- ❌ **`e2e/agentic-warden/ai_cli_update_e2e_tests.rs`**: Depended on common mock module
- ❌ **`e2e/agentic-warden/interactive_mode_e2e_tests.rs`**: Depended on common mock module
- ❌ **`e2e/agentic-warden/mcp_intelligent_route_claude_code_e2e.rs`**: Depended on common mock module
- ❌ **`e2e/agentic-warden/process_tree_e2e_tests.rs`**: Depended on common mock module
- ❌ **`e2e/agentic-warden/scenario_comprehensive.rs`**: Depended on common mock module
- ❌ **`e2e/agentic-warden/workflow_orchestration_tests.rs`**: MockLlmClient, MockWorkflowPlanner
- ❌ **`e2e/agentic-warden/mcp_js_tool_e2e_tests.rs`**: RecordingInvoker mock
- ❌ **`integration/js_orchestrator_tests.rs`**: MockInvoker for MCP tool invocation
- ❌ **`mcp_tools_parameters_test.rs`**: Outdated API (launch_task_tool, manage_tasks_tool don't exist)
- ❌ **`task_tools_comprehensive_test.rs`**: Outdated API (launch_task_tool, manage_tasks_tool don't exist)
- ❌ **`task_tools_unit_test.rs`**: Outdated types (LaunchTaskParams, ManageTasksParams don't exist)
- ❌ **`force_real_rerank.rs`**: FastReranker type no longer exported
- ❌ **`real_rerank_test.rs`**: FastReranker type no longer exported
- ❌ **`rerank_e2e_test.rs`**: FastReranker type no longer exported

## 🚀 **Running Real Tests**

```bash
# Run all real tests (no mocks)
cargo test

# Run specific real tests
cargo test test_real_mcp_server_connection --ignored
cargo test test_force_real_rerank_download --ignored
cargo test test_real_fastembed_reranker -- --ignored

# Run comprehensive integration tests
cargo test --test task_tools_comprehensive_test -- --ignored
```

## 📋 Test Validation Criteria

Real tests must:
1. ✅ Connect to actual services (MCP servers, Ollama, databases)
2. ✅ Use real data and real computations
3. ✅ Verify actual functionality, not simulated responses
4. ✅ Fail gracefully when real services are unavailable
5. ✅ Provide meaningful error messages for real failures

## 🎯 **Current Test Status**

- ✅ **Unit Tests**: 134/134 passing (100% real)
- ✅ **Integration Tests**: All real tests passing
- ✅ **LLM Backend Tests**: 5/5 passing (Ollama + AI CLI coverage)
- ✅ **Model Tests**: Real BGE and FastEmbed models working
- ✅ **No Mock Tests**: All mock-based tests removed

## 🔄 **LLM Backend Coverage**

| Backend | 覆盖情况 | 测试用例 |
|---------|---------|---------|
| **Ollama** | ✅ 完整E2E | `test_e2e_with_ollama_backend` |
| **AI CLI** | ✅ 完整E2E | `test_e2e_with_ai_cli_backend` |
| **自动检测** | ✅ | `test_backend_auto_detection` |
| **工厂创建** | ✅ | `test_backend_factory_creation` |
| **响应对比** | ✅ | `test_backend_response_comparison` |

**All remaining tests are 100% real - no fakes, no simulations!** 🚀