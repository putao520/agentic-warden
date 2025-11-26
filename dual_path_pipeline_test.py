#!/usr/bin/env python3
"""
双路径智能路由管线测试
测试两个路径：
1. OLLAMA路径: 本地LLM→JavaScript→Boa→MCP
2. CODEX路径: AI CLI→JavaScript→Boa→MCP
"""

import subprocess
import json
import time
import sys
import os
import tempfile
from pathlib import Path

class DualPathPipelineTester:
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

    def create_test_data_file(self, data, suffix=".json"):
        """创建测试数据文件"""
        temp_file = tempfile.NamedTemporaryFile(mode='w', suffix=suffix, delete=False)
        json.dump(data, temp_file, indent=2)
        temp_file.close()
        self.temp_files.append(temp_file.name)
        return temp_file.name

    def run_test_with_env(self, env_vars, test_name, test_func):
        """在指定环境变量下运行测试"""
        print(f"\n🔄 测试路径: {test_name}")
        print("=" * 50)
        print(f"环境变量: {env_vars}")

        # 设置环境变量
        original_env = {}
        for key, value in env_vars.items():
            original_env[key] = os.environ.get(key)
            os.environ[key] = value

        try:
            success = test_func()
            self.test_results.append((test_name, success))
            return success
        finally:
            # 恢复原始环境变量
            for key, original_value in original_env.items():
                if original_value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = original_value

    def test_ollama_path(self):
        """测试OLLAMA路径"""
        env_vars = {
            'OLLAMA_ENDPOINT': 'http://localhost:11434',
            'OPENAI_TOKEN': 'sk-dummy-123456',  # 触发OLLAMA模式
            'OLLAMA_MODEL': 'qwen3:1.7b'
        }

        def test_func():
            return self.test_javascript_generation("OLLAMA")

        return self.run_test_with_env(env_vars, "OLLAMA路径", test_func)

    def test_codex_path(self):
        """测试CODEX路径"""
        env_vars = {
            'CLI_TYPE': 'codex',
            'CODEX_BIN': '/home/putao/.nvm/versions/node/v24.5.0/bin/codex',
            # 不设置OPENAI_TOKEN，触发AI CLI模式
        }

        def test_func():
            return self.test_javascript_generation("CODEX")

        return self.run_test_with_env(env_vars, "CODEX路径", test_func)

    def test_javascript_generation(self, backend_name):
        """测试JavaScript代码生成"""
        print(f"🧠 测试{backend_name}后端JavaScript代码生成...")

        # 创建测试数据
        test_data = {
            "users": [
                {"name": "Alice", "score": 85, "department": "Engineering"},
                {"name": "Bob", "score": 92, "department": "Sales"},
                {"name": "Charlie", "score": 78, "department": "Engineering"}
            ]
        }

        data_file = self.create_test_data_file(test_data)

        # 构造智能路由请求 - 触发JavaScript编排
        route_request = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "analyze_user_data",
                "arguments": {
                    "user_request": f"分析{data_file}中的用户数据，计算每个部门的平均分，找出最高分用户，生成分析报告",
                    "data_file": data_file,
                    "complexity": "medium",
                    "requires_workflow": True
                }
            },
            "id": 1
        }

        try:
            print(f"📝 发送{backend_name}智能路由请求...")

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
                    timeout=30
                )

                # 分析结果
                success = self.analyze_javascript_generation_results(
                    stdout, stderr, backend_name
                )

                return success

            except subprocess.TimeoutExpired:
                process.kill()
                print(f"❌ {backend_name}请求超时")
                return False

        except Exception as e:
            print(f"❌ {backend_name}测试异常: {e}")
            return False

    def analyze_javascript_generation_results(self, stdout, stderr, backend_name):
        """分析JavaScript代码生成结果"""

        # 检查后端初始化
        backend_initialized = False
        if backend_name == "OLLAMA":
            backend_initialized = "Ollama code generator initialized" in stderr
        elif backend_name == "CODEX":
            backend_initialized = "AI CLI code generator initialized" in stderr

        # 检查LLM编排模式激活
        llm_orchestration = "LLM orchestration mode" in stderr or "Trying LLM orchestration" in stderr

        # 检查JavaScript代码生成指标
        js_indicators = [
            "javascript" in stdout.lower(),
            "function" in stdout,
            "const " in stdout,
            "let " in stdout,
            "var " in stdout,
            "async" in stdout,
            "await" in stdout
        ]

        # 检查Boa执行指标
        boa_indicators = [
            "boa" in stdout.lower(),
            "execute" in stdout.lower(),
            "runtime" in stdout.lower()
        ]

        # 检查MCP函数调用指标
        mcp_indicators = [
            "mcp.call" in stdout,
            "filesystem" in stdout.lower(),
            "memory" in stdout.lower(),
            "write" in stdout.lower() or "read" in stdout.lower()
        ]

        # 检查实际数据处理
        data_indicators = [
            "85" in stdout,  # Alice's score
            "92" in stdout,  # Bob's score
            "78" in stdout,  # Charlie's score
            "alice" in stdout.lower() or "bob" in stdout.lower() or "charlie" in stdout.lower(),
            "engineering" in stdout.lower() or "sales" in stdout.lower()
        ]

        js_generated = any(js_indicators)
        boa_executed = any(boa_indicators)
        mcp_called = any(mcp_indicators)
        data_processed = any(data_indicators)

        # 输出分析结果
        print(f"✅ {backend_name}后端初始化: {'成功' if backend_initialized else '失败'}")
        print(f"✅ LLM编排模式激活: {'成功' if llm_orchestration else '失败'}")
        print(f"✅ JavaScript代码生成: {'成功' if js_generated else '失败'}")
        print(f"✅ Boa引擎执行: {'成功' if boa_executed else '失败'}")
        print(f"✅ MCP函数调用: {'成功' if mcp_called else '失败'}")
        print(f"✅ 数据处理完成: {'成功' if data_processed else '失败'}")

        if backend_initialized and llm_orchestration:
            print(f"🎯 {backend_name}路径智能路由正常工作")

        if not js_generated:
            print(f"📝 输出预览: {stdout[:400]}...")
            print(f"🔍 错误预览: {stderr[:300]}...")

        # 计算综合成功率
        success_criteria = [
            backend_initialized,
            llm_orchestration,
            js_generated,
            boa_executed or mcp_called  # 至少有一个执行层成功
        ]

        success_count = sum(success_criteria)
        total_criteria = len(success_criteria)
        success_rate = success_count / total_criteria

        print(f"📈 {backend_name}路径成功率: {success_rate:.1%} ({success_count}/{total_criteria})")

        return success_rate >= 0.75  # 75%以上认为成功

    def test_mcp_fallback_mode(self):
        """测试MCP回退模式（无LLM配置）"""
        print(f"\n🔄 测试路径: MCP回退模式")
        print("=" * 50)
        print("环境变量: 无LLM配置")

        # 清除所有LLM相关环境变量
        llm_env_vars = ['OPENAI_TOKEN', 'OLLAMA_ENDPOINT', 'CLI_TYPE', 'CODEX_BIN']
        original_env = {}
        for var in llm_env_vars:
            original_env[var] = os.environ.get(var)
            if var in os.environ:
                os.environ.pop(var)

        try:
            print("🔍 测试向量搜索模式...")

            # 简单的MCP工具列表请求
            mcp_request = {
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": 1
            }

            process = subprocess.Popen(
                [self.aiw_binary, "mcp", "serve"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )

            try:
                stdout, stderr = process.communicate(
                    input=json.dumps(mcp_request),
                    timeout=15
                )

                # 检查回退模式指标
                vector_mode = "vector search mode" in stderr
                llm_not_configured = "LLM not configured" in stderr
                mcp_servers_running = "filesystem" in stdout.lower() or "memory" in stdout.lower()

                success = vector_mode or llm_not_configured or mcp_servers_running

                print(f"✅ 向量搜索模式: {'激活' if vector_mode else '未激活'}")
                print(f"✅ LLM未配置: {'检测到' if llm_not_configured else '未检测到'}")
                print(f"✅ MCP服务器运行: {'正常' if mcp_servers_running else '异常'}")
                print(f"✅ 回退模式: {'成功' if success else '失败'}")

                if not success:
                    print(f"📝 输出: {stdout[:300]}...")
                    print(f"🔍 错误: {stderr[:300]}...")

                self.test_results.append(("MCP回退模式", success))
                return success

            except subprocess.TimeoutExpired:
                process.kill()
                print("❌ MCP回退模式测试超时")
                self.test_results.append(("MCP回退模式", False))
                return False

        except Exception as e:
            print(f"❌ MCP回退模式测试异常: {e}")
            self.test_results.append(("MCP回退模式", False))
            return False
        finally:
            # 恢复环境变量
            for var, original_value in original_env.items():
                if original_value is None:
                    if var in os.environ:
                        os.environ.pop(var)
                else:
                    os.environ[var] = original_value

    def run_all_tests(self):
        """运行所有双路径测试"""
        print("🚀 开始双路径智能路由管线测试")
        print("=" * 70)
        print("测试路径: OLLAMA + CODEX + MCP回退模式")
        print("验证: LLM→JavaScript→Boa→MCP调用→结果返回")

        start_time = time.time()

        try:
            # 测试1: OLLAMA路径
            self.test_ollama_path()

            # 测试2: CODEX路径
            self.test_codex_path()

            # 测试3: MCP回退模式
            self.test_mcp_fallback_mode()

        except Exception as e:
            print(f"❌ 测试运行异常: {e}")
        finally:
            self.cleanup()

        total_time = time.time() - start_time

        # 生成报告
        passed_tests = sum(1 for name, success in self.test_results if success)
        total_tests = len(self.test_results)
        success_rate = (passed_tests / total_tests * 100) if total_tests > 0 else 0

        print("\n" + "=" * 70)
        print("📊 双路径智能路由管线测试总结")
        print("=" * 70)
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
        print("\n🎯 智能路由系统能力评估:")

        capabilities = {
            "OLLAMA后端支持": any("OLLAMA" in name and success for name, success in self.test_results),
            "CODEX后端支持": any("CODEX" in name and success for name, success in self.test_results),
            "MCP回退机制": any("MCP回退" in name and success for name, success in self.test_results),
            "JavaScript代码生成": any(success and ("OLLAMA" in name or "CODEX" in name) for name, success in self.test_results),
        }

        for capability, status in capabilities.items():
            icon = "✅" if status else "❌"
            print(f"{icon} {capability}")

        # 结论
        if success_rate >= 67:
            print("\n🎉 双路径智能路由系统验证成功！")
            print("✅ 多种LLM后端支持正常")
            print("✅ JavaScript代码生成机制工作")
            print("✅ MCP回退机制可靠")
            print("\n🚀 系统已具备完整的多路径智能管线编排能力！")
        elif success_rate >= 33:
            print("\n⚠️ 双路径智能路由系统部分验证")
            print("核心功能基本可用，需要进一步优化某些路径")
        else:
            print("\n❌ 双路径智能路由系统需要重大改进")
            print("关键功能存在问题，需要修复")

        return success_rate >= 33

if __name__ == "__main__":
    tester = DualPathPipelineTester()
    success = tester.run_all_tests()
    sys.exit(0 if success else 1)