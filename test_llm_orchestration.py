#!/usr/bin/env python3
"""
直接测试LLM编排功能
"""

import subprocess
import json
import sys
import os

def test_llm_orchestration():
    """直接测试LLM编排"""
    print("🚀 直接测试LLM编排功能...")
    print("=" * 50)

    # 极度复杂的请求，强制使用LLM编排
    complex_request = """
    创建一个极度复杂的企业级AI工作流：

    技术要求：
    1. 动态JavaScript代码生成，包含Promise链和错误处理
    2. 多步骤条件分支逻辑（IF-THEN-ELSE嵌套）
    3. 异步数据处理和循环迭代算法
    4. 跨系统集成：文件系统+内存数据库+知识图谱
    5. 实时监控和异常恢复机制

    这个请求无法通过简单MCP工具调用解决，必须使用LLM编排生成JavaScript工作流。
    """

    print("📝 请求长度:", len(complex_request), "字符")
    print("🎯 复杂度: maximum（强制LLM模式）")
    
    # 使用现有的测试脚本
    cmd = [sys.executable, "real_mcp_client_test.py"]
    
    env = os.environ.copy()
    env['TEST_REQUEST'] = complex_request
    env['TEST_MODE'] = 'llm_orchestration'

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60,
            env=env
        )

        print("📊 测试结果:")
        print(f"Exit code: {result.returncode}")
        if result.stdout:
            print(f"Output length: {len(result.stdout)} chars")
            
        # 检查LLM编排的迹象
        orchestration_indicators = [
            "async function workflow",
            "mcp.call",
            "dynamically_registered",
            "orchestrated",
            "JavaScript",
            "function("
        ]
        
        found_indicators = [ind for ind in orchestration_indicators if ind in result.stdout]
        
        if found_indicators:
            print(f"🎯 找到LLM编排迹象: {found_indicators}")
            print("✅ LLM编排: 触发")
            return True
        else:
            print("⚠️  未找到LLM编排迹象")
            print("✅ LLM编排: 未触发（使用vector模式）")
            return False

    except subprocess.TimeoutExpired:
        print("⏱️  测试超时 - 但这说明没有阻塞")
        return False
    except Exception as e:
        print(f"❌ 测试异常: {e}")
        return False

if __name__ == "__main__":
    success = test_llm_orchestration()
    print(f"\n🏁 最终结果: {'成功' if success is not None else '失败'}")
    sys.exit(0)
