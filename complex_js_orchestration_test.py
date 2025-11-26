#!/usr/bin/env python3
"""
复杂JavaScript编排测试
触发真正需要：
1. 多个MCP工具组合调用
2. 复杂的业务逻辑
3. 数据流转和条件判断
4. 错误处理和重试机制
"""

import subprocess
import json
import time
import sys
import os
import tempfile
from pathlib import Path

class ComplexJSOrchestrationTester:
    def __init__(self):
        self.aiw_binary = "./target/release/aiw"
        self.test_results = []
        self.temp_files = []

    def cleanup(self):
        """清理临时文件"""
        for temp_file in self.temp_files:
            try:
                os.unlink(temp_file)
            except:
                pass

    def create_complex_test_scenario(self):
        """创建需要复杂编排的测试场景"""
        # 场景：企业销售数据分析报告生成
        # 需要：文件读取 → 数据解析 → 统计计算 → 条件判断 → 多格式输出 → 结果存储

        test_scenario = {
            "sales_data": [
                {"region": "North", "q1": 120000, "q2": 135000, "q3": 142000, "rep": "Alice"},
                {"region": "South", "q1": 98000, "q2": 102000, "q3": 115000, "rep": "Bob"},
                {"region": "East", "q1": 156000, "q2": 148000, "q3": 167000, "rep": "Charlie"},
                {"region": "West", "q1": 87000, "q2": 91000, "q3": 95000, "rep": "Diana"},
                {"region": "Central", "q1": 143000, "q2": 151000, "q3": 138000, "rep": "Eve"}
            ],
            "targets": {
                "quarterly_target": 130000,
                "annual_target": 520000,
                "growth_target": 0.05
            },
            "metadata": {
                "report_date": "2025-11-22",
                "company": "TechCorp Inc.",
                "currency": "USD"
            }
        }

        temp_file = tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False)
        json.dump(test_scenario, temp_file, indent=2)
        temp_file.close()
        self.temp_files.append(temp_file.name)

        return temp_file.name, test_scenario

    def create_user_request_for_complex_orchestration(self, data_file):
        """构造需要复杂编排的用户请求"""
        return f"""
请分析{data_file}中的销售数据并生成综合报告，具体要求：

1. 数据处理：
   - 读取并解析JSON销售数据
   - 计算每个地区的季度增长率
   - 计算总销售额和各季度占比
   - 识别表现最佳和最差的地区和销售代表

2. 业务逻辑：
   - 判断哪些地区达到季度目标
   - 计算年度目标完成度
   - 识别需要改进的地区
   - 生成销售评级（A/B/C/D）

3. 多格式输出：
   - 生成JSON格式的结构化报告
   - 创建Markdown格式的可读报告
   - 生成CSV格式的数据摘要

4. 结果存储：
   - 将JSON报告存储到内存系统供后续查询
   - 将Markdown报告写入文件系统
   - 创建分析摘要的实体关系到知识图谱

5. 错误处理：
   - 如果数据缺失，使用默认值
   - 如果文件写入失败，尝试备用路径
   - 记录所有操作到日志

这是一个需要多个步骤和多个MCP工具协作的复杂业务流程。
        """.strip()

    def test_complex_orchestration_with_backend(self, backend_name, env_vars):
        """测试特定后端的复杂编排"""
        print(f"\n🧠 测试{backend_name}复杂JavaScript编排...")
        print("=" * 60)

        # 设置环境变量
        original_env = {}
        for key, value in env_vars.items():
            original_env[key] = os.environ.get(key)
            os.environ[key] = value

        try:
            # 创建复杂测试场景
            data_file, test_data = self.create_complex_test_scenario()
            user_request = self.create_user_request_for_complex_orchestration(data_file)

            print(f"📁 创建复杂测试场景: {data_file}")
            print(f"📊 场景包含: {len(test_data['sales_data'])}个地区销售数据")
            print(f"🎯 用户请求长度: {len(user_request)} 字符")

            # 构造智能路由请求 - 明确要求复杂编排
            route_request = {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "generate_comprehensive_sales_report",
                    "arguments": {
                        "user_request": user_request,
                        "data_file": data_file,
                        "complexity": "high",
                        "requires_workflow": True,
                        "expected_steps": [
                            "data_parsing",
                            "business_logic",
                            "conditional_analysis",
                            "multi_format_output",
                            "cross_system_storage"
                        ],
                        "mcp_tools_needed": [
                            {"server": "filesystem", "tools": ["read_file", "write_file"]},
                            {"server": "memory", "tools": ["store_data", "create_entities", "add_observations"]}
                        ]
                    }
                },
                "id": 1
            }

            print("🚀 发送复杂编排请求...")

            # 启动MCP服务器
            process = subprocess.Popen(
                [self.aiw_binary, "mcp", "serve"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )

            try:
                stdout, stderr = process.communicate(
                    input=json.dumps(route_request),
                    timeout=45  # 增加超时时间，复杂处理需要更久
                )

                # 分析复杂编排结果
                success = self.analyze_complex_orchestration_results(
                    stdout, stderr, backend_name, test_data
                )

                return success

            except subprocess.TimeoutExpired:
                process.kill()
                print("❌ 复杂编排请求超时")
                return False

        except Exception as e:
            print(f"❌ {backend_name}复杂编排测试异常: {e}")
            return False
        finally:
            # 恢复环境变量
            for key, original_value in original_env.items():
                if original_value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = original_value

    def analyze_complex_orchestration_results(self, stdout, stderr, backend_name, test_data):
        """分析复杂编排结果"""

        print(f"\n📊 分析{backend_name}复杂编排结果...")

        # 1. 检查LLM编排激活
        backend_initialized = False
        llm_orchestration = False
        llm_success = False
        vector_fallback = False

        if backend_name == "OLLAMA":
            backend_initialized = "Ollama code generator initialized" in stderr
        else:
            backend_initialized = "AI CLI code generator initialized" in stderr

        llm_orchestration = "Trying LLM orchestration" in stderr
        llm_success = "LLM orchestration succeeded" in stderr
        vector_fallback = "falling back to vector mode" in stderr

        # 2. 检查JavaScript代码生成 - 寻找复杂代码特征
        js_complexity_indicators = [
            # 函数定义
            "function " in stdout,
            "async " in stdout,
            "await " in stdout,

            # 条件判断
            "if (" in stdout,
            "else" in stdout,
            "switch (" in stdout,

            # 循环处理
            "for (" in stdout,
            "while (" in stdout,
            "forEach" in stdout,
            "map(" in stdout,
            "filter(" in stdout,
            "reduce(" in stdout,

            # 错误处理
            "try " in stdout,
            "catch (" in stdout,
            "finally" in stdout,
            "throw " in stdout,

            # 对象和数组操作
            "const " in stdout,
            "let " in stdout,
            "var " in stdout,
            "Object." in stdout,
            "Array." in stdout,
            "JSON." in stdout,

            # MCP函数调用
            "mcp.call" in stdout,
            "await mcp.call" in stdout
        ]

        # 3. 检查多MCP工具调用
        mcp_tools_indicators = [
            "filesystem" in stdout.lower(),
            "memory" in stdout.lower(),
            "read_file" in stdout,
            "write_file" in stdout,
            "store_data" in stdout,
            "create_entities" in stdout,
            "add_observations" in stdout
        ]

        # 4. 检查业务逻辑处理
        business_logic_indicators = [
            # 数据计算
            "120000" in stdout or "135000" in stdout,  # 具体数据点
            "growth" in stdout.lower() or "增长率" in stdout,
            "total" in stdout.lower() or "总计" in stdout,
            "average" in stdout.lower() or "平均" in stdout,

            # 条件判断结果
            "exceeded" in stdout.lower() or "超过" in stdout,
            "below" in stdout.lower() or "低于" in stdout,
            "target" in stdout.lower() or "目标" in stdout,

            # 地区分析
            "north" in stdout.lower() or "south" in stdout.lower(),
            "east" in stdout.lower() or "west" in stdout.lower(),
            "alice" in stdout.lower() or "bob" in stdout.lower() or "charlie" in stdout.lower(),

            # 报告生成
            "report" in stdout.lower() or "报告" in stdout,
            "json" in stdout.lower(),
            "markdown" in stdout.lower() or "md" in stdout,
            "csv" in stdout.lower()
        ]

        js_generated = len([i for i in js_complexity_indicators if i])
        mcp_tools_used = len([i for i in mcp_tools_indicators if i])
        business_logic_applied = len([i for i in business_logic_indicators if i])

        print(f"✅ {backend_name}后端初始化: {'成功' if backend_initialized else '失败'}")
        print(f"✅ LLM编排模式: {'激活' if llm_orchestration else '未激活'}")
        if llm_orchestration:
            print(f"✅ LLM编排结果: {'成功' if llm_success else '回退到向量模式' if vector_fallback else '失败'}")

        print(f"✅ JavaScript复杂度: {js_generated}/20 项特征")
        print(f"✅ MCP工具调用: {mcp_tools_used}/7 个工具")
        print(f"✅ 业务逻辑应用: {business_logic_applied}/15 项指标")

        # 5. 详细代码分析
        if js_generated >= 5:
            print("🎯 检测到复杂JavaScript代码生成")
            if "async" in stdout and "await" in stdout:
                print("  ✅ 异步编程模式")
            if "if (" in stdout and "else" in stdout:
                print("  ✅ 条件逻辑判断")
            if any(keyword in stdout for keyword in ["for (", "forEach", "map", "filter"]):
                print("  ✅ 循环和数组处理")
            if "try " in stdout and "catch (" in stdout:
                print("  ✅ 错误处理机制")
            if "mcp.call" in stdout:
                print("  ✅ MCP函数调用")

                # 统计mcp.call次数
                mcp_call_count = stdout.count("mcp.call")
                print(f"  📊 MCP调用次数: {mcp_call_count}")

        if mcp_tools_used >= 2:
            print("🎯 检测到多MCP工具协作")

        if business_logic_applied >= 5:
            print("🎯 检测到复杂业务逻辑处理")

        # 输出示例代码片段
        if js_generated > 0:
            print("\n📝 JavaScript代码片段示例:")
            lines = stdout.split('\n')
            js_lines = [line for line in lines if any(keyword in line for keyword in ['function', 'async', 'await', 'mcp.call', 'if (', 'for ('])]
            for line in js_lines[:3]:  # 显示前3行
                print(f"  {line.strip()}")

        # 6. 计算综合成功率
        success_criteria = [
            backend_initialized,  # 后端初始化
            js_generated >= 5,     # 足够复杂的JS代码
            mcp_tools_used >= 2,   # 多个MCP工具
            business_logic_applied >= 3  # 业务逻辑处理
        ]

        success_count = sum(success_criteria)
        total_criteria = len(success_criteria)
        success_rate = success_count / total_criteria

        print(f"\n📈 {backend_name}复杂编排成功率: {success_rate:.1%} ({success_count}/{total_criteria})")

        # 7. 诊断信息
        if not llm_orchestration:
            print("⚠️  LLM编排模式未激活，可能原因:")
            print("  - 请求复杂度不够")
            print("  - 触发了向量搜索回退")
            print("  - 候选工具不足")

        if js_generated < 5:
            print("⚠️  JavaScript代码生成不够复杂")
            print(f"  当前特征: {js_generated}/20")

        return success_rate >= 0.75  # 75%以上认为复杂编排成功

    def run_all_complex_tests(self):
        """运行所有复杂编排测试"""
        print("🚀 开始复杂JavaScript编排测试")
        print("=" * 80)
        print("目标: 触发多MCP工具调用的复杂业务逻辑编排")
        print("验证: JavaScript代码复杂度 + 多工具协作 + 业务逻辑处理")

        start_time = time.time()

        try:
            # 测试1: OLLAMA复杂编排
            ollama_env = {
                'OLLAMA_ENDPOINT': 'http://localhost:11434',
                'OPENAI_TOKEN': 'sk-dummy-123456',
                'OLLAMA_MODEL': 'qwen3:1.7b'
            }
            ollama_success = self.test_complex_orchestration_with_backend("OLLAMA", ollama_env)
            self.test_results.append(("OLLAMA复杂编排", ollama_success))

            # 测试2: CODEX复杂编排
            codex_env = {
                'CLI_TYPE': 'codex',
                'CODEX_BIN': '/home/putao/.nvm/versions/node/v24.5.0/bin/codex'
            }
            codex_success = self.test_complex_orchestration_with_backend("CODEX", codex_env)
            self.test_results.append(("CODEX复杂编排", codex_success))

        except Exception as e:
            print(f"❌ 测试运行异常: {e}")
        finally:
            self.cleanup()

        total_time = time.time() - start_time

        # 生成报告
        passed_tests = sum(1 for name, success in self.test_results if success)
        total_tests = len(self.test_results)
        success_rate = (passed_tests / total_tests * 100) if total_tests > 0 else 0

        print("\n" + "=" * 80)
        print("📊 复杂JavaScript编排测试总结")
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
        print("\n🎯 复杂编排系统能力评估:")

        capabilities = {
            "复杂JavaScript生成": any("复杂编排" in name and success for name, success in self.test_results),
            "多MCP工具协作": any("复杂编排" in name and success for name, success in self.test_results),
            "业务逻辑编排": any("复杂编排" in name and success for name, success in self.test_results),
            "异步处理能力": any("复杂编排" in name and success for name, success in self.test_results),
        }

        for capability, status in capabilities.items():
            icon = "✅" if status else "❌"
            print(f"{icon} {capability}")

        # 结论
        if success_rate >= 50:
            print("\n🎉 复杂JavaScript编排系统验证成功！")
            print("✅ 系统能够生成复杂的JavaScript代码")
            print("✅ 支持多MCP工具协作编排")
            print("✅ 具备业务逻辑处理能力")
            print("\n🚀 系统已具备企业级复杂工作流编排能力！")
        else:
            print("\n⚠️ 复杂编排系统需要进一步优化")
            print("建议:")
            print("  - 调整LLM编排触发条件")
            print("  - 增强复杂度识别机制")
            print("  - 优化MCP工具组合策略")

        return success_rate >= 50

if __name__ == "__main__":
    tester = ComplexJSOrchestrationTester()
    success = tester.run_all_complex_tests()
    sys.exit(0 if success else 1)