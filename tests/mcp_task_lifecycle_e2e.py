#!/usr/bin/env python3
"""
AIW MCP Task Lifecycle E2E Test
Tests the full async task lifecycle using a mock claude script:
  initialize → list_tools → start_task → manage_task(status) →
  manage_task(logs) → manage_task(stop) →
  verify stopped → natural completion → error handling

Uses a fake 'claude' script in a temp dir prepended to PATH so we can
control timing and output without needing a real API key.
"""

import json
import subprocess
import sys
import time
import threading
import select
import os
import stat
import tempfile
import shutil

TIMEOUT = 30

PASS = 0
FAIL = 0

GREEN = "\033[0;32m"
RED = "\033[0;31m"
YELLOW = "\033[0;33m"
NC = "\033[0m"


def log_pass(msg):
    global PASS
    PASS += 1
    print(f"{GREEN}✓ PASS{NC}: {msg}")


def log_fail(msg, detail=""):
    global FAIL
    FAIL += 1
    print(f"{RED}✗ FAIL{NC}: {msg}")
    if detail:
        print(f"  Detail: {detail}")


def log_info(msg):
    print(f"{YELLOW}  info{NC}: {msg}")


def create_mock_claude(tmpdir):
    """Create a fake 'claude' script that simulates a long-running AI task."""
    script_path = os.path.join(tmpdir, "claude")
    with open(script_path, "w") as f:
        f.write("""#!/bin/bash
# Mock claude CLI for E2E testing
# Simulates a task that runs for a while and produces output

# Parse args - look for the prompt (last argument)
PROMPT="${@: -1}"

echo "Mock Claude starting..."
echo "Prompt: $PROMPT"
echo "---"

# Check if prompt asks for quick completion
if echo "$PROMPT" | grep -q "QUICK_EXIT"; then
    echo "Quick mode: completing immediately"
    echo "TASK_COMPLETE_MARKER_12345"
    exit 0
fi

# Simulate a long-running task: output lines over several seconds
for i in $(seq 1 20); do
    echo "Processing step $i of 20..."
    sleep 0.5
done

echo "---"
echo "Task completed successfully."
echo "TASK_COMPLETE_MARKER_12345"
exit 0
""")
    os.chmod(script_path, stat.S_IRWXU | stat.S_IRGRP | stat.S_IXGRP | stat.S_IROTH | stat.S_IXOTH)
    return script_path


class McpClient:
    def __init__(self, extra_env=None):
        env = os.environ.copy()
        # Remove CLAUDECODE env var to avoid nested session detection
        env.pop("CLAUDECODE", None)
        env.pop("CLAUDE_CODE_ENTRYPOINT", None)
        if extra_env:
            env.update(extra_env)

        self.proc = subprocess.Popen(
            ["./target/release/aiw", "mcp", "serve"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
        )
        self._stderr_lines = []
        self._stderr_thread = threading.Thread(target=self._drain_stderr, daemon=True)
        self._stderr_thread.start()

        print("Waiting for server bootstrap...", end="", flush=True)
        for _ in range(20):
            time.sleep(1)
            print(".", end="", flush=True)
            poll = self.proc.poll()
            if poll is not None:
                time.sleep(0.5)
                stderr_tail = "\n".join(self._stderr_lines[-20:])
                raise RuntimeError(
                    f"MCP server exited during bootstrap (code {poll}).\n"
                    f"STDERR (last lines):\n{stderr_tail}"
                )
        print(" ready!")

    def _drain_stderr(self):
        try:
            for line in self.proc.stderr:
                self._stderr_lines.append(line.decode(errors="replace").rstrip())
        except:
            pass

    def send(self, obj):
        body = json.dumps(obj) + "\n"
        self.proc.stdin.write(body.encode())
        self.proc.stdin.flush()

    def recv(self):
        fd = self.proc.stdout.fileno()
        buf = b""
        deadline = time.time() + TIMEOUT
        while time.time() < deadline:
            ready = select.select([self.proc.stdout], [], [], 1)
            if not ready[0]:
                poll = self.proc.poll()
                if poll is not None:
                    stderr_tail = "\n".join(self._stderr_lines[-10:])
                    raise EOFError(
                        f"MCP server exited (code {poll}) while waiting for response.\n"
                        f"STDERR tail:\n{stderr_tail}\n"
                        f"Buffer so far ({len(buf)} bytes): {buf[:200]}"
                    )
                continue
            chunk = os.read(fd, 65536)
            if not chunk:
                poll = self.proc.poll()
                stderr_tail = "\n".join(self._stderr_lines[-10:])
                raise EOFError(
                    f"MCP server closed stdout (process poll={poll}).\n"
                    f"STDERR tail:\n{stderr_tail}\n"
                    f"Buffer so far ({len(buf)} bytes): {buf[:200]}"
                )
            buf += chunk

            while b"\n" in buf:
                line, buf = buf.split(b"\n", 1)
                line = line.strip()
                if line:
                    try:
                        return json.loads(line.decode())
                    except json.JSONDecodeError:
                        continue

        raise TimeoutError(f"Timeout waiting for MCP response (buf={len(buf)} bytes)")

    def call(self, method, params=None, id_=None):
        msg = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            msg["params"] = params
        if id_ is not None:
            msg["id"] = id_
        self.send(msg)
        if id_ is not None:
            return self.recv()

    def tool_call(self, name, arguments=None, id_=1):
        params = {"name": name}
        if arguments is not None:
            params["arguments"] = arguments
        resp = self.call("tools/call", params, id_=id_)
        try:
            text = resp["result"]["content"][0]["text"]
            return json.loads(text), resp
        except (KeyError, IndexError, json.JSONDecodeError):
            return resp, resp

    def close(self):
        try:
            self.proc.stdin.close()
        except:
            pass
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except:
            self.proc.kill()


def main():
    print("=" * 56)
    print(" AIW MCP Task Lifecycle E2E Tests (manage_task unified)")
    print("=" * 56)
    print()

    # Create temp dir with mock claude
    tmpdir = tempfile.mkdtemp(prefix="aiw_e2e_")
    mock_path = create_mock_claude(tmpdir)
    log_info(f"Mock claude at: {mock_path}")

    # Prepend tmpdir to PATH so our mock claude is found first
    env_override = {
        "PATH": tmpdir + ":" + os.environ.get("PATH", ""),
    }

    client = McpClient(extra_env=env_override)
    id_counter = 0

    def next_id():
        nonlocal id_counter
        id_counter += 1
        return id_counter

    try:
        # ─── Test 1: Initialize ───
        print("--- Test 1: MCP Initialize ---")
        resp = client.call(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "e2e-test", "version": "1.0.0"},
            },
            id_=next_id(),
        )

        server_name = resp.get("result", {}).get("serverInfo", {}).get("name", "")
        if server_name == "agentic-warden":
            log_pass(f"Initialize returns server: {server_name}")
        else:
            log_fail("Initialize", json.dumps(resp))

        proto = resp.get("result", {}).get("protocolVersion", "")
        if proto == "2024-11-05":
            log_pass("Protocol version echoed correctly")
        else:
            log_fail("Protocol version", f"got: {proto}")

        client.call("notifications/initialized")
        time.sleep(1)

        # ─── Test 2: List Tools ───
        print()
        print("--- Test 2: List Tools ---")
        resp = client.call("tools/list", {}, id_=next_id())
        tools = [t["name"] for t in resp.get("result", {}).get("tools", [])]

        for tool in ["start_task", "list_tasks", "manage_task"]:
            if tool in tools:
                log_pass(f"Tool '{tool}' is listed")
            else:
                log_fail(f"Tool '{tool}' missing", str(tools))

        # Verify old tools are NOT listed
        for old_tool in ["stop_task", "get_task_logs", "get_task_status"]:
            if old_tool not in tools:
                log_pass(f"Old tool '{old_tool}' correctly removed")
            else:
                log_fail(f"Old tool '{old_tool}' should not exist", str(tools))

        # ─── Test 3: List Tasks (empty) ───
        print()
        print("--- Test 3: List Tasks (should be empty) ---")
        data, raw = client.tool_call("list_tasks", id_=next_id())

        # list_tasks now returns table-formatted text (or "No tasks found.")
        list_text = raw.get("result", {}).get("content", [{}])[0].get("text", "") if isinstance(raw, dict) else str(data)
        if isinstance(data, list) and len(data) == 0:
            log_pass("list_tasks returns empty array initially")
        elif "No tasks found" in list_text:
            log_pass("list_tasks returns 'No tasks found' initially")
        else:
            log_fail("list_tasks not empty", list_text)

        # ═══════════════════════════════════════════════════
        # Scenario A: Long-running task → check alive → stop
        # ═══════════════════════════════════════════════════
        print()
        print("=" * 56)
        print(" Scenario A: Long-running task lifecycle")
        print("=" * 56)

        # ─── Test 4: Start long-running task ───
        print()
        print("--- Test 4: Start Long-Running Task ---")
        data, raw = client.tool_call(
            "start_task",
            {
                "ai_type": "claude",
                "task": "Count from 1 to 20 slowly",
            },
            id_=next_id(),
        )

        task_id = data.get("task_id", "") if isinstance(data, dict) else ""
        task_pid = data.get("pid", 0) if isinstance(data, dict) else 0
        task_status = data.get("status", "") if isinstance(data, dict) else ""

        if task_id:
            log_pass(f"start_task returns task_id: {task_id}")
        else:
            log_fail("start_task no task_id", json.dumps(data))

        if task_pid and task_pid != 0:
            log_pass(f"start_task returns pid: {task_pid}")
        else:
            log_fail("start_task no pid", json.dumps(data))

        if task_status in ("Running", "running"):
            log_pass("start_task status is Running")
        else:
            log_fail("start_task status", f"got: {task_status}")

        # ─── Test 5: manage_task(status) while running ───
        print()
        print("--- Test 5: manage_task(status) While Running ---")
        time.sleep(1)
        if task_id:
            data, _ = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "status"},
                id_=next_id(),
            )

            alive = data.get("process_alive") if isinstance(data, dict) else None
            status = data.get("status", "") if isinstance(data, dict) else ""
            action = data.get("action", "") if isinstance(data, dict) else ""

            if action == "status":
                log_pass("manage_task returns action='status'")
            else:
                log_fail(f"manage_task action mismatch: {action}")

            if alive is True:
                log_pass("Task is ALIVE while running")
            else:
                log_fail(f"Task should be alive but got alive={alive}, status={status}", json.dumps(data))

            if isinstance(data, dict) and data.get("task_id") == task_id:
                log_pass("manage_task(status) returns correct task_id")
            else:
                log_fail("manage_task(status) task_id mismatch", json.dumps(data))

            # Verify status fields are present
            if isinstance(data, dict) and "started_at" in data and data["started_at"]:
                log_pass("manage_task(status) includes started_at")
            else:
                log_fail("manage_task(status) missing started_at", json.dumps(data))
        else:
            log_fail("status check skipped", "no task_id")

        # ─── Test 6: List Tasks while running ───
        print()
        print("--- Test 6: List Tasks While Running ---")
        data, raw = client.tool_call("list_tasks", id_=next_id())

        # list_tasks now returns table-formatted text
        list_text = raw.get("result", {}).get("content", [{}])[0].get("text", "") if isinstance(raw, dict) else str(data)
        if isinstance(data, list) and len(data) >= 1:
            log_pass(f"list_tasks shows {len(data)} task(s)")
            task_ids_in_list = [t.get("task_id", "") for t in data if isinstance(t, dict)]
            if task_id in task_ids_in_list:
                log_pass("Our task_id found in list_tasks")
            else:
                log_fail("Our task_id not in list", str(task_ids_in_list))
        elif "TASK_ID" in list_text and task_id[:10] in list_text:
            # Table format: check header and task_id prefix present
            row_count = list_text.count("\n|") - 1  # subtract header row
            log_pass(f"list_tasks table shows task(s) (rows: {row_count})")
            log_pass("Our task_id found in list_tasks table")
        else:
            log_fail("list_tasks empty after start", list_text)

        # ─── Test 7: manage_task(logs) while running ───
        print()
        print("--- Test 7: manage_task(logs) While Running ---")
        time.sleep(2)  # Let some output accumulate
        if task_id:
            data, _ = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "logs", "tail_lines": 50},
                id_=next_id(),
            )

            action = data.get("action", "") if isinstance(data, dict) else ""
            if action == "logs":
                log_pass("manage_task returns action='logs'")
            else:
                log_fail(f"manage_task action mismatch: {action}")

            if isinstance(data, dict) and data.get("task_id") == task_id:
                log_pass("manage_task(logs) returns correct task_id")
            else:
                log_fail("manage_task(logs) task_id mismatch", json.dumps(data))

            log_file = data.get("log_file", "") if isinstance(data, dict) else ""
            if log_file:
                log_pass("manage_task(logs) returns log_file path")
            else:
                log_fail("manage_task(logs) no log_file", json.dumps(data))

            content = data.get("log_content", "") if isinstance(data, dict) else ""
            if content and "Processing step" in content:
                log_pass(f"Logs show partial progress while running ({len(content)} chars)")
                step_count = content.count("Processing step")
                log_info(f"Can see {step_count} processing steps so far")
            elif content:
                log_pass(f"Logs have content while running ({len(content)} chars)")
            else:
                log_fail("No log content while task is running")
        else:
            log_fail("manage_task(logs) skipped", "no task_id")

        # ─── Test 8: manage_task(logs) full (no tail_lines) ───
        print()
        print("--- Test 8: manage_task(logs) Full Mode ---")
        if task_id:
            data, _ = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "logs"},
                id_=next_id(),
            )

            content = data.get("log_content", "") if isinstance(data, dict) else ""
            if content and len(content) > 0:
                log_pass(f"Full logs retrieved ({len(content)} chars)")
            else:
                log_fail("No content in full log mode", json.dumps(data))
        else:
            log_fail("full logs skipped", "no task_id")

        # ─── Test 9: manage_task(stop) the running task ───
        print()
        print("--- Test 9: manage_task(stop) Running Task ---")
        if task_id:
            # Verify still alive before stopping
            data, _ = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "status"},
                id_=next_id(),
            )
            pre_stop_alive = data.get("process_alive") if isinstance(data, dict) else None
            if pre_stop_alive is True:
                log_info("Confirmed task still alive before stop")
            else:
                log_info(f"Task may have finished (alive={pre_stop_alive})")

            data, raw = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "stop"},
                id_=next_id(),
            )

            action = data.get("action", "") if isinstance(data, dict) else ""
            success = data.get("success") if isinstance(data, dict) else None
            message = data.get("message", "") if isinstance(data, dict) else ""

            if action == "stop":
                log_pass("manage_task returns action='stop'")
            else:
                log_fail(f"manage_task action mismatch: {action}")

            if success is True:
                log_pass(f"manage_task(stop) succeeded: {message}")
            else:
                log_fail("manage_task(stop) failed", json.dumps(raw))

            # stop should also return status fields
            if isinstance(data, dict) and "status" in data:
                log_pass(f"manage_task(stop) includes status: {data.get('status')}")
            else:
                log_fail("manage_task(stop) missing status field")
        else:
            log_fail("manage_task(stop) skipped", "no task_id")

        # ─── Test 10: Verify task is stopped via manage_task(status) ───
        print()
        print("--- Test 10: manage_task(status) After Stop ---")
        time.sleep(1)
        if task_id:
            data, _ = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "status"},
                id_=next_id(),
            )

            alive = data.get("process_alive") if isinstance(data, dict) else None
            status = data.get("status", "") if isinstance(data, dict) else ""

            if alive is False:
                log_pass("Task process is NOT alive after stop")
            else:
                log_fail("Task still alive after stop", json.dumps(data))

            if status not in ("Running", "running"):
                log_pass(f"Task status is no longer Running: {status}")
            else:
                log_fail("Task still Running after stop", json.dumps(data))
        else:
            log_fail("post-stop status skipped", "no task_id")

        # ─── Test 11: Logs preserved after stop ───
        print()
        print("--- Test 11: Logs Preserved After Stop ---")
        if task_id:
            data, _ = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "logs", "tail_lines": 100},
                id_=next_id(),
            )

            content = data.get("log_content", "") if isinstance(data, dict) else ""
            if content and len(content) > 0:
                log_pass(f"Logs preserved after stop ({len(content)} chars)")
                snippet = content[:200].replace("\n", "\\n")
                log_info(f"Log snippet: {snippet}...")
            else:
                log_fail("No log content after stop", json.dumps(data))
        else:
            log_fail("post-stop logs skipped", "no task_id")

        # ═══════════════════════════════════════════════════
        # Scenario B: Short task → natural completion → get results
        # ═══════════════════════════════════════════════════
        print()
        print("=" * 56)
        print(" Scenario B: Natural completion lifecycle")
        print("=" * 56)

        # ─── Test 12: Start short task ───
        print()
        print("--- Test 12: Start Short Task ---")
        data, raw = client.tool_call(
            "start_task",
            {
                "ai_type": "claude",
                "task": "QUICK_EXIT and say hello",
            },
            id_=next_id(),
        )

        task2_id = data.get("task_id", "") if isinstance(data, dict) else ""
        if task2_id:
            log_pass(f"Short task started: {task2_id}")
        else:
            log_fail("Short task start failed", json.dumps(data))

        # ─── Test 13: Poll until natural completion ───
        print()
        print("--- Test 13: Wait for Natural Completion ---")
        if task2_id:
            completed = False
            for i in range(15):
                time.sleep(1)
                data, _ = client.tool_call(
                    "manage_task",
                    {"task_id": task2_id, "action": "status"},
                    id_=next_id(),
                )
                alive = data.get("process_alive") if isinstance(data, dict) else None
                status = data.get("status", "") if isinstance(data, dict) else ""
                if alive is False:
                    completed = True
                    log_pass(f"Task completed naturally after ~{i+1}s (status={status})")
                    break

            if not completed:
                log_fail("Task did not complete within 15s")
                client.tool_call(
                    "manage_task",
                    {"task_id": task2_id, "action": "stop"},
                    id_=next_id(),
                )
        else:
            log_fail("poll skipped", "no task2_id")

        # ─── Test 14: Get completed task results ───
        print()
        print("--- Test 14: Get Completed Task Logs ---")
        if task2_id:
            data, _ = client.tool_call(
                "manage_task",
                {"task_id": task2_id, "action": "logs", "tail_lines": 100},
                id_=next_id(),
            )

            content = data.get("log_content", "") if isinstance(data, dict) else ""
            if content and "TASK_COMPLETE_MARKER_12345" in content:
                log_pass("Completed task logs contain expected marker")
            elif content and len(content) > 10:
                log_pass(f"Completed task has log content ({len(content)} chars)")
                log_info("Marker not found but content exists")
            else:
                log_fail("Completed task has no/minimal log content", json.dumps(data))

            if content:
                snippet = content[:300].replace("\n", "\\n")
                log_info(f"Log: {snippet}")
        else:
            log_fail("completed task logs skipped", "no task2_id")

        # ═══════════════════════════════════════════════════
        # Scenario C: Edge cases and error handling
        # ═══════════════════════════════════════════════════
        print()
        print("=" * 56)
        print(" Scenario C: Edge cases and error handling")
        print("=" * 56)

        # ─── Test 15: List tasks shows history ───
        print()
        print("--- Test 15: List Tasks Shows History ---")
        data, raw = client.tool_call("list_tasks", id_=next_id())

        # list_tasks now returns table-formatted text
        list_text = raw.get("result", {}).get("content", [{}])[0].get("text", "") if isinstance(raw, dict) else str(data)
        if isinstance(data, list) and len(data) >= 2:
            log_pass(f"list_tasks shows {len(data)} tasks (history preserved)")
        elif "TASK_ID" in list_text:
            # Count data rows in table (lines starting with "| " that aren't the header)
            table_rows = [l for l in list_text.split("\n") if l.startswith("|") and "TASK_ID" not in l]
            if len(table_rows) >= 2:
                log_pass(f"list_tasks table shows {len(table_rows)} tasks (history preserved)")
            else:
                log_fail(f"list_tasks should show 2+ tasks, got {len(table_rows)} rows", list_text)
        else:
            log_fail("list_tasks should show 2+ tasks", list_text)

        # ─── Test 16: Invalid task_id ───
        print()
        print("--- Test 16: Error - Invalid task_id ---")
        data, raw = client.tool_call(
            "manage_task",
            {"task_id": "nonexistent-uuid-12345", "action": "status"},
            id_=next_id(),
        )

        is_error = False
        try:
            is_error = raw.get("result", {}).get("isError", False)
        except:
            pass
        has_error_key = isinstance(raw, dict) and "error" in raw
        error_in_text = False
        data_str = str(data).lower() if data else ""
        if any(w in data_str for w in ["not found", "error", "invalid"]):
            error_in_text = True

        if is_error or has_error_key or error_in_text:
            log_pass("Invalid task_id returns error")
        else:
            log_fail("No error for invalid task_id", json.dumps(raw))

        # ─── Test 17: Invalid action ───
        print()
        print("--- Test 17: Error - Invalid action ---")
        if task_id:
            data, raw = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "invalid_action"},
                id_=next_id(),
            )

            is_error = False
            try:
                is_error = raw.get("result", {}).get("isError", False)
            except:
                pass
            has_error_key = isinstance(raw, dict) and "error" in raw
            data_str = str(data).lower() if data else ""
            error_in_text = any(w in data_str for w in ["unknown action", "error", "invalid"])

            if is_error or has_error_key or error_in_text:
                log_pass("Invalid action returns error")
            else:
                log_fail("No error for invalid action", json.dumps(raw))
        else:
            log_fail("invalid action test skipped", "no task_id")

        # ─── Test 18: Stop already-stopped task ───
        print()
        print("--- Test 18: Stop Already-Stopped Task ---")
        if task_id:
            data, raw = client.tool_call(
                "manage_task",
                {"task_id": task_id, "action": "stop"},
                id_=next_id(),
            )

            success = data.get("success") if isinstance(data, dict) else None
            message = data.get("message", "") if isinstance(data, dict) else ""

            if success is True:
                log_pass(f"Re-stopping already-stopped task succeeds gracefully: {message}")
            else:
                log_fail("Re-stopping task failed", json.dumps(raw))
        else:
            log_fail("re-stop skipped", "no task_id")

        # ─── Test 19: Stop naturally-completed task ───
        print()
        print("--- Test 19: Stop Naturally-Completed Task ---")
        if task2_id:
            data, raw = client.tool_call(
                "manage_task",
                {"task_id": task2_id, "action": "stop"},
                id_=next_id(),
            )

            success = data.get("success") if isinstance(data, dict) else None
            if success is True:
                log_pass("Stopping naturally-completed task succeeds gracefully")
            else:
                log_fail("Stopping completed task failed", json.dumps(raw))
        else:
            log_fail("stop completed task skipped", "no task2_id")

    finally:
        client.close()
        # Cleanup temp dir
        shutil.rmtree(tmpdir, ignore_errors=True)

    # ─── Summary ───
    total = PASS + FAIL
    print()
    print("=" * 56)
    print(f" Results: {GREEN}{PASS} passed{NC}, {RED}{FAIL} failed{NC}, {total} total")
    print("=" * 56)

    sys.exit(1 if FAIL > 0 else 0)


if __name__ == "__main__":
    main()
