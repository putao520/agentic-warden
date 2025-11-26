#!/usr/bin/env python3
"""
直接测试CODEX调用和真实输出
"""

import subprocess
import json
import sys
import time
import os

def test_direct_codex():
    """直接测试CODEX调用"""
    print("🔧 直接测试CODEX调用...")
    print("=" * 50)

    # 创建一个明确需要LLM编排的请求
    workflow_request = """
    Create a complex JavaScript workflow that requires multi-step orchestration:

    1. Read a CSV file from the filesystem
    2. Process the data with statistical calculations
    3. Generate a JSON report with aggregated results
    4. Save the report to a different location
    5. Send a notification when complete

    This workflow must include:
    - Async/await patterns
    - Error handling with try/catch blocks
    - Multiple MCP tool calls in sequence
    - Data transformation between steps
    - Promise chains for asynchronous operations

    Generate the complete JavaScript code for this workflow.
    """

    print(f"📝 请求长度: {len(workflow_request)} 字符")

    # 直接调用Agentic-Warden的智能路由功能
    test_script = '''
import json
import sys
import os

# 设置环境变量强制使用CODEX
os.environ["CLI_TYPE"] = "codex"
os.environ["CODEX_BIN"] = "/home/putao/.nvm/versions/node/v24.5.0/bin/codex"

# 导入智能路由模块
sys.path.insert(0, "/home/putao/code/rust/agentic-warden")
from src.mcp_routing import IntelligentRouter

async def test_llm_orchestration():
    print("🚀 初始化智能路由...")

    try:
        router = await IntelligentRouter.initialize()
        print("✅ 智能路由初始化成功")

        # 构造请求
        request_data = {
            "user_request": """ + json.dumps(workflow_request) + """,
            "max_candidates": 1,
            "execution_mode": "dynamic",
            "session_id": "test_direct_codex"
        }

        print("📤 发送LLM编排请求...")

        # 调用智能路由
        result = await router.intelligent_route(request_data)

        print(f"✅ 请求完成")
        print(f"成功: {result.success}")
        print(f"置信度: {result.confidence}")
        print(f"动态注册: {result.dynamically_registered}")

        if result.success:
            if result.dynamically_registered:
                print("🎯 LLM编排触发: SUCCESS")
                print(f"消息: {result.message}")
                if result.selected_tool:
                    print(f"工具名称: {result.selected_tool.tool_name}")
                return True
            else:
                print("⚠️ 使用向量搜索模式")
                if result.selected_tool:
                    print(f"选择的工具: {result.selected_tool.tool_name}")
                return False
        else:
            print(f"❌ 请求失败: {result.message}")
            return False

    except Exception as e:
        print(f"❌ 异常: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    import asyncio
    success = asyncio.run(test_llm_orchestration())
    sys.exit(0 if success else 1)
'''

    # 保存测试脚本
    with open("temp_direct_test.py", "w") as f:
        f.write(test_script)

    try:
        # 设置环境变量并运行测试
        env = os.environ.copy()
        env["CLI_TYPE"] = "codex"
        env["CODEX_BIN"] = "/home/putao/.nvm/versions/node/v24.5.0/bin/codex"
        env["RUST_LOG"] = "debug"

        print("🔧 运行直接CODEX测试...")

        result = subprocess.run(
            [sys.executable, "temp_direct_test.py"],
            capture_output=True,
            text=True,
            timeout=300,  # 5分钟超时
            env=env,
            cwd="/home/putao/code/rust/agentic-warden"
        )

        print("📊 测试结果:")
        print("STDOUT:")
        print(result.stdout)

        if result.stderr:
            print("STDERR:")
            print(result.stderr)

        print(f"Exit code: {result.returncode}")

        # 检查是否有真实的JavaScript输出
        if "async function" in result.stdout or "mcp.call" in result.stdout:
            print("✅ 发现真实的JavaScript/LLM输出")
            return True
        elif "LLM编排触发: SUCCESS" in result.stdout:
            print("✅ LLM编排成功触发")
            return True
        else:
            print("⚠️ 没有检测到LLM编排")
            return False

    except subprocess.TimeoutExpired:
        print("⏱️ 测试超时")
        return False
    except Exception as e:
        print(f"❌ 测试异常: {e}")
        return False
    finally:
        # 清理临时文件
        if os.path.exists("temp_direct_test.py"):
            os.remove("temp_direct_test.py")

if __name__ == "__main__":
    success = test_direct_codex()
    print(f"\n🏁 最终结果: {'成功' if success else '失败'}")
    sys.exit(0 if success else 1)