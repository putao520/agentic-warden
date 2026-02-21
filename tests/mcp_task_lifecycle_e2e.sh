#!/usr/bin/env bash
# MCP Task Lifecycle E2E Test Script
# Tests: initialize → list_tools → start_task → list_tasks → get_task_status → get_task_logs → stop_task
#
# Usage: ./tests/mcp_task_lifecycle_e2e.sh

set -eo pipefail

AIW_CMD="${AIW_CMD:-./target/release/aiw mcp serve}"
TIMEOUT=10
PASS=0
FAIL=0
TOTAL=0

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_pass() { ((PASS++)); ((TOTAL++)); echo -e "${GREEN}✓ PASS${NC}: $1"; }
log_fail() { ((FAIL++)); ((TOTAL++)); echo -e "${RED}✗ FAIL${NC}: $1"; echo "  Detail: $2"; }

# Use named pipes for bidirectional communication
TMPDIR_E2E=$(mktemp -d)
PIPE_IN="$TMPDIR_E2E/mcp_in"
PIPE_OUT="$TMPDIR_E2E/mcp_out"
mkfifo "$PIPE_IN" "$PIPE_OUT"

# Start aiw mcp with pipes
$AIW_CMD < "$PIPE_IN" > "$PIPE_OUT" 2>/dev/null &
MCP_PID=$!

# Open file descriptors for read/write
exec 3>"$PIPE_IN"   # write to MCP stdin
exec 4<"$PIPE_OUT"  # read from MCP stdout

cleanup() {
    exec 3>&- 2>/dev/null || true
    exec 4<&- 2>/dev/null || true
    kill "$MCP_PID" 2>/dev/null || true
    wait "$MCP_PID" 2>/dev/null || true
    rm -rf "$TMPDIR_E2E"
}
trap cleanup EXIT

# Send a JSON-RPC request via Content-Length framing
send_rpc() {
    local json="$1"
    local len=${#json}
    printf "Content-Length: %d\r\n\r\n%s" "$len" "$json" >&3
}

read_rpc() {
    local line=""
    local content_length=0

    # Read headers until empty line
    while IFS= read -r -t "$TIMEOUT" line <&4; do
        line="${line%%$'\r'}"
        if [[ "$line" == Content-Length:* ]]; then
            content_length="${line#Content-Length: }"
        fi
        if [[ -z "$line" ]]; then
            break
        fi
    done

    if [[ "$content_length" -gt 0 ]]; then
        # Read exactly content_length bytes
        local body=""
        IFS= read -r -n "$content_length" -t "$TIMEOUT" body <&4
        echo "$body"
    fi
}

echo "========================================"
echo " AIW MCP Task Lifecycle E2E Tests"
echo "========================================"
echo ""

# Give server a moment to start
sleep 0.5

# ─── Test 1: Initialize ───
echo "--- Test 1: MCP Initialize ---"
send_rpc '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e-test","version":"1.0.0"}}}'
RESP=$(read_rpc)

if echo "$RESP" | jq -e '.result.serverInfo.name == "agentic-warden"' >/dev/null 2>&1; then
    log_pass "Initialize returns agentic-warden server info"
else
    log_fail "Initialize" "$RESP"
fi

PROTO=$(echo "$RESP" | jq -r '.result.protocolVersion' 2>/dev/null)
if [[ "$PROTO" == "2024-11-05" ]]; then
    log_pass "Protocol version echoed correctly"
else
    log_fail "Protocol version" "got: $PROTO"
fi

# Send initialized notification
send_rpc '{"jsonrpc":"2.0","method":"notifications/initialized"}'
sleep 0.5

# ─── Test 2: List Tools ───
echo ""
echo "--- Test 2: List Tools ---"
send_rpc '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
RESP=$(read_rpc)

TOOL_NAMES=$(echo "$RESP" | jq -r '.result.tools[].name' 2>/dev/null)

for tool in start_task list_tasks stop_task get_task_logs get_task_status; do
    if echo "$TOOL_NAMES" | grep -q "^${tool}$"; then
        log_pass "Tool '$tool' is listed"
    else
        log_fail "Tool '$tool' missing from tools/list" "$TOOL_NAMES"
    fi
done

# ─── Test 3: List Tasks (empty) ───
echo ""
echo "--- Test 3: List Tasks (should be empty) ---"
send_rpc '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_tasks","arguments":{}}}'
RESP=$(read_rpc)

CONTENT=$(echo "$RESP" | jq -r '.result.content[0].text' 2>/dev/null)
TASK_COUNT=$(echo "$CONTENT" | jq 'length' 2>/dev/null)

if [[ "$TASK_COUNT" == "0" ]]; then
    log_pass "list_tasks returns empty array initially"
else
    log_fail "list_tasks not empty" "$CONTENT"
fi

# ─── Test 4: Start Task ───
echo ""
echo "--- Test 4: Start Task ---"
send_rpc '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"start_task","arguments":{"ai_type":"claude","task":"echo hello from e2e test"}}}'
RESP=$(read_rpc)

CONTENT=$(echo "$RESP" | jq -r '.result.content[0].text' 2>/dev/null)
TASK_ID=$(echo "$CONTENT" | jq -r '.task_id' 2>/dev/null)
TASK_PID=$(echo "$CONTENT" | jq -r '.pid' 2>/dev/null)
TASK_STATUS=$(echo "$CONTENT" | jq -r '.status' 2>/dev/null)

if [[ -n "$TASK_ID" && "$TASK_ID" != "null" ]]; then
    log_pass "start_task returns task_id: $TASK_ID"
else
    log_fail "start_task no task_id" "$CONTENT"
fi

if [[ -n "$TASK_PID" && "$TASK_PID" != "null" && "$TASK_PID" != "0" ]]; then
    log_pass "start_task returns pid: $TASK_PID"
else
    log_fail "start_task no pid" "$CONTENT"
fi

if [[ "$TASK_STATUS" == "Running" || "$TASK_STATUS" == "running" ]]; then
    log_pass "start_task status is Running"
else
    log_fail "start_task status" "got: $TASK_STATUS"
fi

# Wait a moment for the task to register
sleep 1

# ─── Test 5: List Tasks (should have 1) ───
echo ""
echo "--- Test 5: List Tasks (should have 1+) ---"
send_rpc '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"list_tasks","arguments":{}}}'
RESP=$(read_rpc)

CONTENT=$(echo "$RESP" | jq -r '.result.content[0].text' 2>/dev/null)
TASK_COUNT=$(echo "$CONTENT" | jq 'length' 2>/dev/null)

if [[ "$TASK_COUNT" -ge 1 ]]; then
    log_pass "list_tasks shows $TASK_COUNT task(s)"
else
    log_fail "list_tasks empty after start" "$CONTENT"
fi

# ─── Test 6: Get Task Status ───
echo ""
echo "--- Test 6: Get Task Status ---"
if [[ -n "$TASK_ID" && "$TASK_ID" != "null" ]]; then
    send_rpc "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"tools/call\",\"params\":{\"name\":\"get_task_status\",\"arguments\":{\"task_id\":\"$TASK_ID\"}}}"
    RESP=$(read_rpc)

    CONTENT=$(echo "$RESP" | jq -r '.result.content[0].text' 2>/dev/null)
    STATUS_TASK_ID=$(echo "$CONTENT" | jq -r '.task_id' 2>/dev/null)
    PROCESS_ALIVE=$(echo "$CONTENT" | jq -r '.process_alive' 2>/dev/null)

    if [[ "$STATUS_TASK_ID" == "$TASK_ID" ]]; then
        log_pass "get_task_status returns correct task_id"
    else
        log_fail "get_task_status task_id mismatch" "expected $TASK_ID, got $STATUS_TASK_ID"
    fi

    if [[ "$PROCESS_ALIVE" == "true" || "$PROCESS_ALIVE" == "false" ]]; then
        log_pass "get_task_status reports process_alive: $PROCESS_ALIVE"
    else
        log_fail "get_task_status process_alive" "$CONTENT"
    fi
else
    log_fail "get_task_status skipped" "no task_id from start_task"
fi

# ─── Test 7: Get Task Logs ───
echo ""
echo "--- Test 7: Get Task Logs ---"
if [[ -n "$TASK_ID" && "$TASK_ID" != "null" ]]; then
    send_rpc "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"tools/call\",\"params\":{\"name\":\"get_task_logs\",\"arguments\":{\"task_id\":\"$TASK_ID\",\"tail_lines\":20}}}"
    RESP=$(read_rpc)

    CONTENT=$(echo "$RESP" | jq -r '.result.content[0].text' 2>/dev/null)
    LOG_TASK_ID=$(echo "$CONTENT" | jq -r '.task_id' 2>/dev/null)
    LOG_FILE=$(echo "$CONTENT" | jq -r '.log_file' 2>/dev/null)

    if [[ "$LOG_TASK_ID" == "$TASK_ID" ]]; then
        log_pass "get_task_logs returns correct task_id"
    else
        log_fail "get_task_logs task_id mismatch" "expected $TASK_ID, got $LOG_TASK_ID"
    fi

    if [[ -n "$LOG_FILE" && "$LOG_FILE" != "null" ]]; then
        log_pass "get_task_logs returns log_file path: $LOG_FILE"
    else
        log_fail "get_task_logs no log_file" "$CONTENT"
    fi
else
    log_fail "get_task_logs skipped" "no task_id from start_task"
fi

# ─── Test 8: Stop Task ───
echo ""
echo "--- Test 8: Stop Task ---"
if [[ -n "$TASK_ID" && "$TASK_ID" != "null" ]]; then
    send_rpc "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"tools/call\",\"params\":{\"name\":\"stop_task\",\"arguments\":{\"task_id\":\"$TASK_ID\"}}}"
    RESP=$(read_rpc)

    CONTENT=$(echo "$RESP" | jq -r '.result.content[0].text' 2>/dev/null)
    STOP_SUCCESS=$(echo "$CONTENT" | jq -r '.success' 2>/dev/null)
    STOP_MSG=$(echo "$CONTENT" | jq -r '.message' 2>/dev/null)

    if [[ "$STOP_SUCCESS" == "true" ]]; then
        log_pass "stop_task succeeded: $STOP_MSG"
    else
        # Task may have already exited, which is also acceptable
        if echo "$STOP_MSG" | grep -qi "already\|not found\|exited\|completed"; then
            log_pass "stop_task: task already finished (acceptable): $STOP_MSG"
        else
            log_fail "stop_task" "$CONTENT"
        fi
    fi
else
    log_fail "stop_task skipped" "no task_id from start_task"
fi

# ─── Test 9: Get Task Status after stop ───
echo ""
echo "--- Test 9: Task Status After Stop ---"
sleep 1
if [[ -n "$TASK_ID" && "$TASK_ID" != "null" ]]; then
    send_rpc "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/call\",\"params\":{\"name\":\"get_task_status\",\"arguments\":{\"task_id\":\"$TASK_ID\"}}}"
    RESP=$(read_rpc)

    CONTENT=$(echo "$RESP" | jq -r '.result.content[0].text' 2>/dev/null)
    FINAL_STATUS=$(echo "$CONTENT" | jq -r '.status' 2>/dev/null)
    FINAL_ALIVE=$(echo "$CONTENT" | jq -r '.process_alive' 2>/dev/null)

    if [[ "$FINAL_ALIVE" == "false" ]]; then
        log_pass "Task process is no longer alive after stop"
    else
        log_fail "Task still alive after stop" "$CONTENT"
    fi

    if [[ "$FINAL_STATUS" != "Running" && "$FINAL_STATUS" != "running" ]]; then
        log_pass "Task status is no longer Running: $FINAL_STATUS"
    else
        log_fail "Task still Running after stop" "$CONTENT"
    fi
else
    log_fail "post-stop status skipped" "no task_id"
fi

# ─── Test 10: Invalid task_id ───
echo ""
echo "--- Test 10: Error Handling - Invalid task_id ---"
send_rpc '{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"get_task_status","arguments":{"task_id":"nonexistent-uuid-12345"}}}'
RESP=$(read_rpc)

IS_ERROR=$(echo "$RESP" | jq -e '.result.isError // .error' 2>/dev/null)
if [[ -n "$IS_ERROR" ]]; then
    log_pass "get_task_status with invalid task_id returns error"
else
    # Check if the content itself indicates an error
    CONTENT=$(echo "$RESP" | jq -r '.result.content[0].text' 2>/dev/null)
    if echo "$CONTENT" | grep -qi "not found\|error\|invalid"; then
        log_pass "get_task_status with invalid task_id returns error message"
    else
        log_fail "No error for invalid task_id" "$RESP"
    fi
fi

# ─── Summary ───
echo ""
echo "========================================"
echo -e " Results: ${GREEN}${PASS} passed${NC}, ${RED}${FAIL} failed${NC}, ${TOTAL} total"
echo "========================================"

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
