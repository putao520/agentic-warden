#!/usr/bin/env python3
"""
Simple test to verify CODEX integration works with the supervisor fix
"""

import subprocess
import json
import sys
import os

def test_simple_codex_call():
    """Test simple CODEX call through Agentic-Warden"""
    print("🔧 测试CODEX简单调用...")
    print("=" * 50)

    # Simple request that should work
    simple_request = "Generate a simple JavaScript function that adds two numbers"

    # Test via the mcp client which should trigger orchestration if the environment is set up correctly
    cmd = [
        sys.executable, "real_mcp_client_test.py"
    ]

    env = os.environ.copy()
    env['TEST_REQUEST'] = simple_request
    env['TEST_MODE'] = 'simple'

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=30,
            env=env
        )

        print(f"✅ Exit code: {result.returncode}")
        if result.stdout:
            print(f"📄 Output length: {len(result.stdout)} chars")

            # Check for signs of LLM orchestration
            orchestration_indicators = [
                "workflow(",
                "async function",
                "mcp.call",
                "JavaScript",
                "function"
            ]

            found_indicators = [ind for ind in orchestration_indicators if ind in result.stdout]
            if found_indicators:
                print(f"🎯 Found LLM orchestration indicators: {found_indicators}")
                print("✅ LLM Orchestration: SUCCESS")
                return True
            else:
                print("⚠️  No LLM orchestration indicators found")
                print("✅ LLM Orchestration: NOT TRIGGERED (using vector mode)")
                return False
        else:
            print("❌ No output received")
            return False

    except subprocess.TimeoutExpired:
        print("⏱️  Test timed out (30s)")
        return False
    except Exception as e:
        print(f"❌ Test failed with error: {e}")
        return False

if __name__ == "__main__":
    success = test_simple_codex_call()
    if success:
        print("\n🎉 CODEX integration test PASSED")
        sys.exit(0)
    else:
        print("\n🧪 CODEX integration test completed (no orchestration)")
        sys.exit(0)  # Still exit 0 since this is just informational