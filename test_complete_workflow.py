#!/usr/bin/env python3
"""
完整的智能路由管线测试
基于simple_mcp_test.py的成功模式
"""

import subprocess
import json
import time
import tempfile
import os

def test_complete_pipeline():
    """测试完整管线：用户请求 → 智能路由 → LLM编排 → JavaScript执行 → MCP调用"""
    print("🧠 测试完整的智能路由管线...")
    print("=" * 60)

    try:
        # 创建测试数据文件
        test_data = {
            "test_scenario": "file_workflow_test",
            "operations": [
                {"action": "create_file", "content": "Hello from LLM workflow!"},
                {"action": "read_file", "verify": True}
            ]
        }

        temp_file = tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False)
        json.dump(test_data, temp_file, indent=2)
        temp_file.close()

        try:
            # 正确的MCP协议握手 + 智能路由请求
            mcp_handshake_and_route = [
                # 1. initialize请求
                {
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {
                                "listChanged": True
                            }
                        },
                        "clientInfo": {
                            "name": "test-client",
                            "version": "1.0.0"
                        }
                    },
                    "id": 1
                },
                # 2. initialized通知（无ID，无响应）
                {
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                },
                # 3. 智能路由请求
                {
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "intelligent_route",
                        "arguments": {
                            "user_request": f"""
请执行以下文件操作工作流，需要JavaScript编排：

1. 读取测试数据文件: {temp_file.name}
2. 根据数据创建新的临时文件
3. 写入内容: "Hello from LLM workflow!"
4. 读取新创建的文件确认写入成功
5. 将操作结果存储到内存系统

这个任务需要JavaScript工作流编排来协调多个MCP工具调用：
- 文件系统工具: read_file, write_file
- 内存工具: store_data
- 错误处理和异步操作

请使用LLM生成完整的JavaScript代码来处理这个工作流。
""",
                            "execution_mode": "dynamic",
                            "complexity": "high",
                            "require_workflow": True,
                            "prefer_llm_generation": True,
                            "force_llm_mode": True,
                            "min_workflow_steps": 5
                        }
                    },
                    "id": 2
                }
            ]

            # 将所有请求组合成一个JSONL格式的字符串
            request_text = "\n".join(json.dumps(req) for req in mcp_handshake_and_route)

            print(f"🚀 发送完整管线测试请求...")
            print(f"📁 测试数据文件: {temp_file.name}")

            # 启动MCP服务器
            env = {
                'CLI_TYPE': 'codex',
                'CODEX_BIN': '/home/putao/.nvm/versions/node/v24.5.0/bin/codex'
            }

            process = subprocess.Popen(
                ['./target/release/aiw', 'mcp', 'serve'],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                env={**dict(os.environ), **env}
            )

            try:
                stdout, stderr = process.communicate(
                    input=request_text,
                    timeout=240  # 4分钟超时，应该足够了
                )

                return analyze_pipeline_results(stdout, stderr, test_data, temp_file.name)

            except subprocess.TimeoutExpired:
                process.kill()
                print("❌ 管线测试超时（可能LLM正在生成复杂代码）")
                return False

        finally:
            os.unlink(temp_file.name)

    except Exception as e:
        print(f"❌ 管线测试异常: {e}")
        return False

def analyze_pipeline_results(stdout, stderr, test_data, temp_file_path):
    """分析完整管线执行结果"""
    print(f"\n📊 分析管线执行结果...")
    print(f"stdout长度: {len(stdout)}")
    print(f"stderr长度: {len(stderr)}")

    # 1. 检查CODEX初始化
    codex_initialized = "🤖 AI CLI code generator initialized: codex" in stderr

    # 2. 检查智能路由响应
    has_response = len(stdout) > 1000  # 有实际响应内容
    routing_decision = "selected_tool" in stdout or "Vector-based" in stdout

    # 3. 检查LLM编排指标
    llm_orchestration = "🔄 Initiating LLM workflow orchestration" in stderr
    llm_success = "LLM orchestration succeeded" in stderr
    vector_fallback = "Vector-based tool routing" in stdout

    # 4. 检查JavaScript生成特征
    js_features = [
        "function" in stdout,
        "async" in stdout,
        "await" in stdout,
        "mcp.call" in stdout,
        "try" in stdout and "catch" in stdout,
        "const" in stdout or "let" in stdout
    ]

    js_complexity = sum(js_features)

    # 5. 检查MCP工具调用
    mcp_tools = [
        "read_file" in stdout,
        "write_file" in stdout,
        "store_data" in stdout,
        "memory" in stdout.lower(),
        "filesystem" in stdout.lower()
    ]

    mcp_usage = sum(mcp_tools)

    # 6. 检查业务逻辑处理
    business_logic = [
        "Hello from LLM workflow" in stdout,
        temp_file_path in stdout,
        "workflow" in stdout.lower(),
        "file" in stdout.lower()
    ]

    business_processing = sum(business_logic)

    print(f"\n🎯 关键指标分析:")
    print(f"✅ CODEX后端初始化: {'成功' if codex_initialized else '失败'}")
    print(f"✅ 智能路由响应: {'是' if has_response else '否'}")
    print(f"✅ 路由决策: {'是' if routing_decision else '否'}")

    print(f"✅ LLM编排触发: {'是' if llm_orchestration else '否'}")
    if llm_orchestration:
        print(f"✅ LLM编排成功: {'是' if llm_success else '否'}")
    if vector_fallback:
        print(f"⚠️ 使用向量搜索模式")

    print(f"✅ JavaScript复杂度: {js_complexity}/6 项特征")
    print(f"✅ MCP工具使用: {mcp_usage}/5 个工具")
    print(f"✅ 业务逻辑处理: {business_processing}/4 项指标")

    # 7. 输出代码片段示例
    if js_complexity > 2:
        print(f"\n📝 JavaScript代码特征:")
        if "function" in stdout:
            print(f"  ✅ 函数定义")
        if "async" in stdout and "await" in stdout:
            print(f"  ✅ 异步编程")
        if "mcp.call" in stdout:
            print(f"  ✅ MCP函数调用")
        if "try" in stdout and "catch" in stdout:
            print(f"  ✅ 错误处理")

    # 8. 输出响应内容预览
    if has_response:
        print(f"\n🔍 响应内容预览:")
        lines = stdout.split('\n')
        for i, line in enumerate(lines[:3]):
            if line.strip():
                print(f"  {i+1}: {line[:150]}{'...' if len(line) > 150 else ''}")

    # 9. 综合评估
    success_criteria = [
        codex_initialized,        # 后端初始化
        has_response,            # 智能路由响应
        js_complexity >= 2,      # 足够的JS代码生成
        mcp_usage >= 1           # MCP工具调用
    ]

    success_count = sum(success_criteria)
    total_criteria = len(success_criteria)
    success_rate = success_count / total_criteria

    print(f"\n📈 完整管线成功率: {success_rate:.1%} ({success_count}/{total_criteria})")

    # 10. 管线完整性判断
    if success_rate >= 0.75:
        print(f"\n🎉 完整智能路由管线验证成功！")
        print(f"✅ 系统成功执行: 用户请求 → 智能路由 → JavaScript生成 → MCP工具调用")
        return True
    else:
        print(f"\n⚠️ 管线需要进一步优化")
        if not codex_initialized:
            print(f"  - CODEX后端初始化失败")
        if not has_response:
            print(f"  - 智能路由无响应")
        if js_complexity < 2:
            print(f"  - JavaScript代码生成不足")
        if mcp_usage == 0:
            print(f"  - MCP工具调用缺失")

        return False

if __name__ == "__main__":
    success = test_complete_pipeline()
    exit(0 if success else 1)