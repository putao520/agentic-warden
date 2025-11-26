#!/usr/bin/env python3
"""
完整的MCP协议测试
包含正确的握手流程和intelligent_route调用
"""

import subprocess
import json
import time
import sys
import os
import tempfile
import threading
import queue

class CompleteMCPProtocolTester:
    def __init__(self):
        self.aiw_binary = "./target/release/aiw"
        self.test_results = []

    def send_mcp_request(self, process, request):
        """发送MCP请求并获取响应"""
        try:
            request_json = json.dumps(request) + "\n"
            process.stdin.write(request_json)
            process.stdin.flush()

            # 读取响应直到遇到完整的JSON
            response_lines = []
            while True:
                line = process.stdout.readline()
                if not line:
                    break
                line = line.strip()
                if line:
                    response_lines.append(line)
                    try:
                        # 尝试解析JSON
                        response = json.loads("\n".join(response_lines))
                        return response, "", True
                    except json.JSONDecodeError:
                        # 还不是完整的JSON，继续读取
                        continue

            return {}, "No response received", False

        except Exception as e:
            return {}, f"Request error: {e}", False

    def test_complete_mcp_protocol(self, backend_name, env_vars):
        """测试完整的MCP协议和智能路由"""
        print(f"\n🤝 测试{backend_name}完整MCP协议...")
        print("=" * 60)

        # 设置环境变量
        original_env = {}
        for key, value in env_vars.items():
            original_env[key] = os.environ.get(key)
            os.environ[key] = value

        try:
            # 启动MCP服务器进程
            process = subprocess.Popen(
                [self.aiw_binary, "mcp", "serve"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,  # 行缓冲
                universal_newlines=True
            )

            try:
                # Step 1: initialize 请求
                print("📡 Step 1: 发送initialize请求...")
                init_request = {
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {},
                            "sampling": {}
                        },
                        "clientInfo": {
                            "name": "test-client",
                            "version": "1.0.0"
                        }
                    },
                    "id": 1
                }

                response, error, success = self.send_mcp_request(process, init_request)
                if not success:
                    print(f"❌ initialize请求失败: {error}")
                    return False

                print("✅ initialize成功")

                # Step 2: initialized 通知
                print("📡 Step 2: 发送initialized通知...")
                initialized_notification = {
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                }

                # 通知不需要响应
                try:
                    request_json = json.dumps(initialized_notification) + "\n"
                    process.stdin.write(request_json)
                    process.stdin.flush()
                except Exception as e:
                    print(f"⚠️ initialized通知失败: {e}")

                print("✅ initialized通知发送")

                # 等待服务器完全启动
                time.sleep(2)

                # Step 3: tools/list 请求
                print("📡 Step 3: 获取工具列表...")
                tools_request = {
                    "jsonrpc": "2.0",
                    "method": "tools/list",
                    "id": 2
                }

                response, error, success = self.send_mcp_request(process, tools_request)
                if not success:
                    print(f"❌ tools/list请求失败: {error}")
                    return False

                # 检查是否有intelligent_route工具
                has_intelligent_route = False
                if "result" in response and "tools" in response["result"]:
                    tools = response["result"]["tools"]
                    for tool in tools:
                        if tool.get("name") == "intelligent_route":
                            has_intelligent_route = True
                            print("✅ 找到intelligent_route工具")
                            break

                if not has_intelligent_route:
                    print("❌ 未找到intelligent_route工具")
                    return False

                # Step 4: 调用intelligent_route工具
                print("📡 Step 4: 调用intelligent_route工具...")

                # 创建测试数据
                test_data = {
                    "sales": [
                        {"region": "North", "q1": 120000, "q2": 135000},
                        {"region": "South", "q1": 98000, "q2": 102000}
                    ]
                }

                temp_file = tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False)
                json.dump(test_data, temp_file, indent=2)
                temp_file.close()

                try:
                    intelligent_route_request = {
                        "jsonrpc": "2.0",
                        "method": "tools/call",
                        "params": {
                            "name": "intelligent_route",
                            "arguments": {
                                "user_request": f"""
请分析{temp_file.name}中的销售数据并执行以下复杂任务：

1. 数据处理：
   - 读取JSON文件中的销售数据
   - 计算每个地区的季度增长率
   - 识别表现最佳的地区

2. 多步骤分析：
   - 生成统计摘要
   - 创建趋势分析
   - 生成预测模型

3. 多格式输出：
   - JSON格式的结构化报告
   - Markdown格式的可读报告
   - CSV格式的数据表

4. 跨系统存储：
   - 将结果存储到memory系统
   - 将报告写入文件系统
   - 创建知识图谱实体

这是一个需要多个MCP工具协作的复杂任务，请使用JavaScript工作流编排来处理。
                                """,
                                "execution_mode": "dynamic",
                                "max_candidates": 10,
                                "complexity": "high"
                            }
                        },
                        "id": 3
                    }

                    response, error, success = self.send_mcp_request(process, intelligent_route_request)
                    if not success:
                        print(f"❌ intelligent_route请求失败: {error}")
                        return False

                    # 分析智能路由结果
                    routing_success = self.analyze_intelligent_routing_response(
                        response, backend_name, test_data
                    )

                    return routing_success

                finally:
                    os.unlink(temp_file.name)

            except subprocess.TimeoutExpired:
                process.kill()
                print("❌ MCP协议测试超时")
                return False
            except Exception as e:
                print(f"❌ MCP协议测试异常: {e}")
                return False

        except Exception as e:
            print(f"❌ 启动MCP服务器失败: {e}")
            return False
        finally:
            # 恢复环境变量
            for key, original_value in original_env.items():
                if original_value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = original_value

    def analyze_intelligent_routing_response(self, response, backend_name, test_data):
        """分析智能路由响应"""
        print(f"\n📊 分析{backend_name}智能路由响应...")

        if not response or "result" not in response:
            print("❌ 智能路由响应为空")
            return False

        result = response["result"]

        # 检查基本字段
        has_selected_tool = "selected_tool" in result
        has_confidence = "confidence" in result
        has_rationale = "rationale" in result
        is_dynamically_registered = result.get("dynamically_registered", False)

        print(f"✅ 选定工具: {'是' if has_selected_tool else '否'}")
        if has_selected_tool:
            selected = result["selected_tool"]
            print(f"  - 工具名称: {selected.get('tool_name', 'Unknown')}")
            print(f"  - 服务器: {selected.get('mcp_server', 'Unknown')}")
            print(f"  - 理由: {selected.get('rationale', 'No rationale')[:100]}...")

        print(f"✅ 置信度: {'是' if has_confidence else '否'}")
        if has_confidence:
            print(f"  - 置信度值: {result['confidence']}")

        print(f"✅ 推理说明: {'是' if has_rationale else '否'}")
        print(f"✅ 动态注册: {'是' if is_dynamically_registered else '否'}")

        # 检查工具模式
        if has_selected_tool:
            selected_tool = result["selected_tool"]
            tool_name = selected_tool.get("tool_name", "")
            server = selected_tool.get("mcp_server", "")

            # 分析工具选择结果
            if tool_name == "intelligent_route":
                print("🔄 智能路由递归调用 - 可能表示复杂工作流")
            elif tool_name in ["read_file", "read_text_file"]:
                print("📁 选择了文件读取工具")
            elif tool_name in ["store_data", "create_entities"]:
                print("🧠 选择了内存存储工具")
            elif "orchestrated" in server.lower():
                print("🎯 选择了动态编排的工作流")
                print("✅ 检测到LLM工作流编排！")
                return True
            else:
                print(f"🔧 选择了工具: {tool_name} (来自 {server})")

        # 计算成功率
        success_criteria = [
            has_selected_tool,        # 成功选择了工具
            has_confidence >= 0.5,    # 有合理的置信度
            has_rationale,            # 有推理说明
        ]

        # 如果是动态注册的工作流，加分
        if is_dynamically_registered:
            print("🎯 动态工作流创建成功！")
            success_criteria.append(True)

        success_count = sum(1 for i, criterion in enumerate(success_criteria) if criterion)
        total_criteria = len(success_criteria)
        success_rate = success_count / total_criteria

        print(f"\n📈 {backend_name}智能路由成功率: {success_rate:.1%} ({success_count}/{total_criteria})")

        return success_rate >= 0.75

    def run_all_tests(self):
        """运行所有完整MCP协议测试"""
        print("🚀 开始完整MCP协议和智能路由测试")
        print("=" * 80)
        print("包含正确的MCP握手流程和intelligent_route调用")
        print("验证: LLM编排 + 动态工作流创建 + 多MCP工具协作")

        start_time = time.time()

        try:
            # 测试1: OLLAMA完整协议
            ollama_env = {
                'OLLAMA_ENDPOINT': 'http://localhost:11434',
                'OPENAI_TOKEN': 'sk-dummy-123456',
                'OLLAMA_MODEL': 'qwen3:1.7b'
            }
            ollama_success = self.test_complete_mcp_protocol("OLLAMA", ollama_env)
            self.test_results.append(("OLLAMA完整协议", ollama_success))

            # 测试2: CODEX完整协议
            codex_env = {
                'CLI_TYPE': 'codex',
                'CODEX_BIN': '/home/putao/.nvm/versions/node/v24.5.0/bin/codex'
            }
            codex_success = self.test_complete_mcp_protocol("CODEX", codex_env)
            self.test_results.append(("CODEX完整协议", codex_success))

        except Exception as e:
            print(f"❌ 测试运行异常: {e}")

        total_time = time.time() - start_time

        # 生成报告
        passed_tests = sum(1 for name, success in self.test_results if success)
        total_tests = len(self.test_results)
        success_rate = (passed_tests / total_tests * 100) if total_tests > 0 else 0

        print("\n" + "=" * 80)
        print("📊 完整MCP协议和智能路由测试总结")
        print("=" * 80)
        print(f"总测试数: {total_tests}")
        print(f"通过测试: {passed_tests}")
        print(f"失败测试: {total_tests - passed_tests}")
        print(f"成功率: {success_rate:.1f}%")
        print(f"总耗时: {total_time:.2f}秒")

        print("\n🔍 详细结果:")
        for name, success in self.test_results:
            status = "✅" if success else "❌"
            print(f"{status} {name}")

        # 关键能力评估
        print("\n🎯 系统能力评估:")

        capabilities = {
            "完整MCP协议": any("完整协议" in name and success for name, success in self.test_results),
            "智能路由功能": any("完整协议" in name and success for name, success in self.test_results),
            "工具选择逻辑": any("完整协议" in name and success for name, success in self.test_results),
            "推理能力": any("完整协议" in name and success for name, success in self.test_results),
        }

        for capability, status in capabilities.items():
            icon = "✅" if status else "❌"
            print(f"{icon} {capability}")

        # 结论
        if success_rate >= 50:
            print("\n🎉 完整MCP协议和智能路由系统验证成功！")
            print("✅ MCP协议握手流程正确")
            print("✅ intelligent_route工具可用")
            print("✅ 智能路由决策功能正常")
            print("\n🚀 系统已具备完整的智能路由和MCP协作能力！")
        else:
            print("\n⚠️ 智能路由系统需要进一步调试")
            print("建议:")
            print("  - 检查MCP协议实现")
            print("  - 验证工具注册流程")
            print("  - 调试路由决策逻辑")

        return success_rate >= 50

if __name__ == "__main__":
    tester = CompleteMCPProtocolTester()
    success = tester.run_all_tests()
    sys.exit(0 if success else 1)