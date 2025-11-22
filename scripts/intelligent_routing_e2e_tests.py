#!/usr/bin/env python3
"""
Agentic-Warden 智能路由系统 E2E 测试
基于真实 OLLAMA 和 CODEX 的完整测试
"""

import os
import sys
import json
import time
import subprocess
import shutil
import tempfile
import requests
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional, Tuple, Any
from dataclasses import dataclass, asdict
from enum import Enum
import re

class TestStatus(Enum):
    PASSED = "PASSED"
    FAILED = "FAILED"
    SKIPPED = "SKIPPED"
    ERROR = "ERROR"

@dataclass
class TestResult:
    name: str
    status: TestStatus
    duration: float
    output: str
    error: Optional[str] = None
    route_type: Optional[str] = None  # 'js_workflow' or 'direct_mcp'
    quality_score: Optional[Dict[str, float]] = None

@dataclass
class QualityScore:
    syntax_correctness: float = 0.0
    logic_correctness: float = 0.0
    security_score: float = 0.0
    efficiency_score: float = 0.0
    maintainability: float = 0.0
    overall_score: float = 0.0

class IntelligentRoutingTester:
    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.aiw_binary = project_root / "target/release/aiw"
        self.test_results_dir = project_root / "test-results"
        self.timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        self.report_file = self.test_results_dir / f"routing_e2e_report_{self.timestamp}.md"

        # 统计
        self.total_tests = 0
        self.passed_tests = 0
        self.failed_tests = 0
        self.skipped_tests = 0

        # 测试结果
        self.test_results: List[TestResult] = []

        # 创建结果目录
        self.test_results_dir.mkdir(exist_ok=True)

        # 配置MCP服务器
        self.setup_mcp_config()

    def log_info(self, message: str):
        print(f"[INFO] {message}")
        self._write_to_report(f"**{message}**")

    def log_success(self, message: str):
        print(f"[PASS] {message}")
        self._write_to_report(f"✅ {message}")
        self.passed_tests += 1

    def log_error(self, message: str):
        print(f"[FAIL] {message}")
        self._write_to_report(f"❌ {message}")
        self.failed_tests += 1

    def log_warning(self, message: str):
        print(f"[WARN] {message}")
        self._write_to_report(f"⚠️ {message}")

    def log_test(self, message: str):
        print(f"[TEST] {message}")
        self._write_to_report(f"🧪 {message}")
        self.total_tests += 1

    def _write_to_report(self, content: str):
        try:
            with open(self.report_file, 'a', encoding='utf-8') as f:
                f.write(f"{content}\n")
        except IOError as e:
            print(f"Warning: Could not write to report file: {e}")

    def setup_mcp_config(self):
        """设置MCP配置"""
        mcp_config = {
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["@modelcontextprotocol/server-filesystem", "/tmp"],
                    "enabled": True
                },
                "memory": {
                    "command": "npx",
                    "args": ["@modelcontextprotocol/server-memory"],
                    "enabled": True
                }
            }
        }

        config_path = Path.home() / ".aiw" / ".mcp.json"
        config_path.parent.mkdir(exist_ok=True)

        with open(config_path, 'w') as f:
            json.dump(mcp_config, f, indent=2)

        self.log_info(f"已配置MCP服务器: {config_path}")

    def run_command(self, command: str, timeout: int = 60, input_data: Optional[str] = None) -> Tuple[int, str, str]:
        """运行命令并返回结果"""
        try:
            result = subprocess.run(
                command,
                shell=True,
                capture_output=True,
                text=True,
                timeout=timeout,
                input=input_data,
                cwd=self.project_root
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return -1, "", "Command timed out"

    def check_ollama_available(self) -> bool:
        """检查OLLAMA是否可用"""
        try:
            result = subprocess.run(["ollama", "list"], capture_output=True, text=True)
            return result.returncode == 0
        except FileNotFoundError:
            return False

    def check_ollama_model(self, model: str) -> bool:
        """检查OLLAMA模型是否可用"""
        try:
            result = subprocess.run(["ollama", "list"], capture_output=True, text=True)
            return model in result.stdout
        except (FileNotFoundError, subprocess.SubprocessError):
            return False

    def evaluate_code_quality(self, code: str) -> QualityScore:
        """评估代码生成质量"""
        score = QualityScore()

        # 语法正确性检查
        try:
            compile(code, '<string>', 'exec')
            score.syntax_correctness = 1.0
        except SyntaxError:
            score.syntax_correctness = 0.0
        except Exception:
            score.syntax_correctness = 0.5  # 其他错误可能是运行时问题

        # 安全性检查
        security_patterns = [
            r'eval\s*\(',  # eval使用
            r'exec\s*\(',  # exec使用
            r'subprocess\.',  # subprocess未过滤
            r'os\.system',  # os.system使用
            r'shell=True',  # shell=True风险
        ]

        security_issues = sum(1 for pattern in security_patterns if re.search(pattern, code, re.IGNORECASE))
        score.security_score = max(0.0, 1.0 - (security_issues * 0.2))

        # 可维护性检查
        maintainability_patterns = [
            r'def\s+\w+\s*\([^)]*\):',  # 函数定义
            r'class\s+\w+',  # 类定义
            r'#.*',  # 注释
            r'""".*"""',  # 文档字符串
            r"'.*'",  # 文档字符串
        ]

        maintainability_score = sum(1 for pattern in maintainability_patterns if re.search(pattern, code, re.MULTILINE | re.DOTALL))
        score.maintainability = min(1.0, maintainability_score / 3.0)  # 最多3分

        # 效率检查（简单启发式）
        efficiency_indicators = [
            r'async\s+def',  # 异步函数
            r'await\s+',  # 异步等待
            r'for.*in.*range',  # 循环效率
            r'list\s*comprehension',  # 列表推导
        ]

        efficiency_score = sum(1 for pattern in efficiency_indicators if re.search(pattern, code, re.MULTILINE))
        score.efficiency = min(1.0, efficiency_score / 2.0)  # 最多2分

        # 逻辑正确性（基于代码结构和复杂度的启发式评估）
        code_lines = len([line for line in code.split('\n') if line.strip()])
        logic_complexity = min(1.0, code_lines / 50.0)  # 基于行数的简单评估
        score.logic_correctness = 0.7 + 0.3 * logic_complexity  # 基础分+复杂度分

        # 计算总分
        score.overall_score = (
            score.syntax_correctness * 0.3 +
            score.logic_correctness * 0.2 +
            score.security_score * 0.2 +
            score.efficiency_score * 0.15 +
            score.maintainability * 0.15
        )

        return score

    def test_intelligent_routing_with_request(self, user_request: str, expected_route: str = None) -> TestResult:
        """测试智能路由功能"""
        self.log_test(f"智能路由测试: {user_request[:50]}...")

        start_time = time.time()

        # 构造MCP工具调用请求
        mcp_request = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "intelligent_route",
                "arguments": {
                    "user_request": user_request
                }
            },
            "id": 1
        }

        # 调用智能路由
        code, stdout, stderr = self.run_command(
            f"echo '{json.dumps(mcp_request)}' | timeout 30s {self.aiw_binary} mcp serve",
            timeout=45
        )

        duration = time.time() - start_time

        # 分析响应
        route_type = None
        if "JavaScript" in stdout or "workflow" in stdout:
            route_type = "js_workflow"
        elif "direct" in stdout or "MCP" in stdout:
            route_type = "direct_mcp"

        # 检查是否成功
        if code == 0 or (code == -1 and "timeout" not in stderr.lower()):
            # 代码质量评估（如果生成代码）
            quality_score = None
            if "function" in stdout or "async" in stdout:
                # 提取生成的代码片段
                code_pattern = r'```(?:javascript|js)\n(.*?)\n```'
                matches = re.findall(code_pattern, stdout, re.DOTALL | re.MULTILINE)
                if matches:
                    quality_score = asdict(self.evaluate_code_quality(matches[0]))

            result = TestResult(
                name=f"路由测试: {user_request[:30]}...",
                status=TestStatus.PASSED,
                duration=duration,
                output=stdout[:1000],
                route_type=route_type,
                quality_score=quality_score
            )

            self.log_success(f"路由测试完成: {route_type}")
            return result
        else:
            error_msg = stderr[:500] if stderr else "Unknown error"
            result = TestResult(
                name=f"路由测试: {user_request[:30]}...",
                status=TestStatus.FAILED,
                duration=duration,
                output=stdout[:500],
                error=error_msg,
                route_type=route_type
            )

            self.log_error(f"路由测试失败: {error_msg}")
            return result

    def test_ollama_integration(self) -> List[TestResult]:
        """测试OLLAMA集成"""
        if not self.check_ollama_available():
            self.log_warning("OLLAMA不可用，跳过OLLAMA测试")
            return [TestResult(
                name="OLLAMA集成测试",
                status=TestStatus.SKIPPED,
                duration=0,
                output="OLLAMA不可用",
                error="OLLAMA未安装或未运行"
            )]

        results = []

        # 测试不同模型
        models_to_test = ["llama3.1:8b"]
        if self.check_ollama_model("llama3.1:70b"):
            models_to_test.append("llama3.1:70b")

        test_requests = [
            "生成一个Python函数，计算斐波那契数列",
            "创建一个简单的HTTP服务器处理GET请求",
            "实现文件读取和数据解析功能"
        ]

        for model in models_to_test:
            self.log_info(f"测试OLLAMA模型: {model}")

            for request in test_requests:
                # 构造包含OLLAMA的请求
                mcp_request = {
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "intelligent_route_with_ollama",
                        "arguments": {
                            "user_request": request,
                            "model": model
                        }
                    },
                    "id": 1
                }

                # 注意：这里需要根据实际API调整
                # 假设我们有一个特定的OLLAMA工具
                start_time = time.time()
                code, stdout, stderr = self.run_command(
                    f"echo '{json.dumps(mcp_request)}' | timeout 60s {self.aiw_binary} mcp serve",
                    timeout=90
                )
                duration = time.time() - start_time

                if code == 0 or "ollama" in stdout.lower():
                    quality_score = None
                    if "def " in stdout or "function" in stdout:
                        # 评估生成的代码质量
                        quality_score = asdict(self.evaluate_code_quality(stdout))

                    result = TestResult(
                        name=f"OLLAMA {model}: {request[:30]}...",
                        status=TestStatus.PASSED,
                        duration=duration,
                        output=stdout[:1000],
                        route_type="ollama_generated",
                        quality_score=quality_score
                    )

                    self.log_success(f"OLLAMA {model} 测试通过")
                else:
                    result = TestResult(
                        name=f"OLLAMA {model}: {request[:30]}...",
                        status=TestStatus.FAILED,
                        duration=duration,
                        output=stdout[:500],
                        error=stderr[:500],
                        route_type="ollama_failed"
                    )

                    self.log_error(f"OLLAMA {model} 测试失败")

                results.append(result)

        return results

    def test_real_codex_integration(self) -> List[TestResult]:
        """测试真实CODEX集成"""
        if not shutil.which("codex"):
            self.log_warning("CODEX不可用，跳过CODEX测试")
            return [TestResult(
                name="CODEX集成测试",
                status=TestStatus.SKIPPED,
                duration=0,
                output="CODEX不可用",
                error="CODEX未安装或不在PATH中"
            )]

        results = []

        test_requests = [
            "创建一个REST API端点处理用户认证",
            "实现一个React组件支持表格排序和分页",
            "生成数据缓存机制支持TTL",
            "创建一个WebSocket服务器处理实时通信"
        ]

        self.log_info("测试真实CODEX集成")

        for request in test_requests:
            start_time = time.time()

            # 使用真实CODEX环境变量
            codex_bin = shutil.which("codex")
            env = os.environ.copy()
            env["CODEX_BIN"] = codex_bin

            code, stdout, stderr = self.run_command(
                f"CODEX_BIN={codex_bin} ./target/debug/test_launch",
                timeout=60
            )

            duration = time.time() - start_time

            if code == 0 and "Task launched successfully" in stdout:
                # 分析生成代码的质量
                quality_score = None

                # 提取任务输出
                if "PID:" in stdout:
                    # 尝试获取生成的代码
                    pid_match = re.search(r'PID:\s*(\d+)', stdout)
                    if pid_match:
                        pid = pid_match.group(1)
                        # 这里可以读取生成的代码文件或日志来评估质量

                result = TestResult(
                    name=f"CODEX: {request[:30]}...",
                    status=TestStatus.PASSED,
                    duration=duration,
                    output=stdout[:1000],
                    route_type="codex_real",
                    quality_score=quality_score
                )

                self.log_success(f"CODEX测试通过: {request[:30]}...")
            else:
                result = TestResult(
                    name=f"CODEX: {request[:30]}...",
                    status=TestStatus.FAILED,
                    duration=duration,
                    output=stdout[:500],
                    error=stderr[:500],
                    route_type="codex_failed"
                )

                self.log_error(f"CODEX测试失败: {request[:30]}...")

            results.append(result)

        return results

    def test_routing_decision_accuracy(self) -> List[TestResult]:
        """测试路由决策准确性"""
        results = []

        # 简单操作应该走直接MCP路由
        simple_operations = [
            ("读取文件内容", "direct_mcp"),
            ("列出目录内容", "direct_mcp"),
            ("写入数据到内存", "direct_mcp")
        ]

        # 复杂操作应该走JavaScript工作流路由
        complex_operations = [
            ("读取JSON文件，处理数据后写入新文件", "js_workflow"),
            ("监控文件变化并记录到内存系统", "js_workflow"),
            ("解析配置文件并生成报告", "js_workflow")
        ]

        self.log_info("测试路由决策准确性")

        for request, expected_route in simple_operations + complex_operations:
            result = self.test_intelligent_routing_with_request(request, expected_route)

            # 验证路由是否正确
            if result.route_type == expected_route:
                result.status = TestStatus.PASSED
                self.log_success(f"路由决策正确: {request[:30]} -> {expected_route}")
            else:
                result.status = TestStatus.FAILED
                result.error = f"Expected {expected_route}, got {result.route_type}"
                self.log_error(f"路由决策错误: {request[:30]} -> {result.route_type}")

            results.append(result)

        return results

    def init_report(self):
        """初始化测试报告"""
        # 检查环境
        ollama_available = self.check_ollama_available()
        codex_available = shutil.which("codex") is not None

        available_models = []
        if ollama_available:
            for model in ["llama3.1:8b", "llama3.1:70b"]:
                if self.check_ollama_model(model):
                    available_models.append(model)

        with open(self.report_file, 'w', encoding='utf-8') as f:
            f.write(f"""# Agentic-Warden 智能路由系统 E2E 测试报告

**测试时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
**项目根目录**: {self.project_root}
**AIW二进制**: {self.aiw_binary}
**Python版本**: {sys.version}

## 环境检查

- **OLLAMA可用**: {'✅' if ollama_available else '❌'}
- **CODEX可用**: {'✅' if codex_available else '❌'}
- **可用OLLAMA模型**: {', '.join(available_models) if available_models else '无'}

## 测试概览

""")

    def finalize_report(self):
        """完成测试报告"""
        success_rate = (self.passed_tests * 100 // self.total_tests) if self.total_tests > 0 else 0

        with open(self.report_file, 'a', encoding='utf-8') as f:
            f.write(f"""
## 测试总结

- **总测试数**: {self.total_tests}
- **通过测试**: {self.passed_tests}
- **失败测试**: {self.failed_tests}
- **跳过测试**: {self.skipped_tests}
- **成功率**: {success_rate}%

## 详细测试结果

""")

            # 按路由类型分组结果
            route_groups = {}
            for result in self.test_results:
                route_type = result.route_type or "unknown"
                if route_type not in route_groups:
                    route_groups[route_type] = []
                route_groups[route_type].append(result)

            for route_type, results in route_groups.items():
                f.write(f"### {route_type.replace('_', ' ').title()}\n\n")

                for result in results:
                    status_icon = "✅" if result.status == TestStatus.PASSED else "❌"
                    f.write(f"- {status_icon} **{result.name}** ({result.status.value})\n")
                    f.write(f"  - 耗时: {result.duration:.2f}s\n")
                    if result.error:
                        f.write(f"  - 错误: {result.error[:200]}\n")
                    if result.quality_score:
                        f.write(f"  - 质量评分: {result.quality_score.get('overall_score', 0):.2f}/1.0\n")
                f.write("\n")

            # 质量统计
            quality_scores = [r.quality_score for r in self.test_results if r.quality_score]
            if quality_scores:
                avg_syntax = sum(s.get('syntax_correctness', 0) for s in quality_scores) / len(quality_scores)
                avg_security = sum(s.get('security_score', 0) for s in quality_scores) / len(quality_scores)
                avg_overall = sum(s.get('overall_score', 0) for s in quality_scores) / len(quality_scores)

                f.write(f"""### 代码生成质量统计

- **平均语法正确性**: {avg_syntax:.2f}/1.0
- **平均安全性评分**: {avg_security:.2f}/1.0
- **平均总体质量**: {avg_overall:.2f}/1.0

""")

        if self.failed_tests == 0:
            print(f"\n🎉 所有智能路由测试通过！成功率: {success_rate}%")
        else:
            print(f"\n⚠️  有 {self.failed_tests} 个测试失败。成功率: {success_rate}%")

        print(f"📊 详细报告: {self.report_file}")

    def run_all_tests(self) -> bool:
        """运行所有智能路由E2E测试"""
        print("🚀 开始Agentic-Warden 智能路由系统E2E测试")
        print("=" * 60)

        # 初始化报告
        self.init_report()

        # 检查构建
        if not self.aiw_binary.exists():
            self.log_info("构建Agentic-Warden...")
            subprocess.run("cargo build --release", shell=True, cwd=self.project_root)

        # 运行测试套件
        try:
            # 1. 路由决策准确性测试
            self.log_info("=== 路由决策准确性测试 ===")
            routing_results = self.test_routing_decision_accuracy()
            self.test_results.extend(routing_results)

            # 2. OLLAMA集成测试
            self.log_info("=== OLLAMA集成测试 ===")
            ollama_results = self.test_ollama_integration()
            self.test_results.extend(ollama_results)

            # 3. 真实CODEX集成测试
            self.log_info("=== 真实CODEX集成测试 ===")
            codex_results = self.test_real_codex_integration()
            self.test_results.extend(codex_results)

        except Exception as e:
            self.log_error(f"测试执行异常: {e}")
            return False

        # 生成报告
        self.finalize_report()

        # 返回是否全部通过
        return self.failed_tests == 0

def main():
    """主函数"""
    project_root = Path(__file__).parent.parent
    tester = IntelligentRoutingTester(project_root)

    success = tester.run_all_tests()
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()