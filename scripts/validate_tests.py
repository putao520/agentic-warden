#!/usr/bin/env python3
"""
Agentic-Warden 测试验证脚本
验证测试的完整性和质量
"""

import os
import sys
import json
import subprocess
import argparse
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from enum import Enum

class TestStatus(Enum):
    PASSED = "passed"
    FAILED = "failed"
    SKIPPED = "skipped"

@dataclass
class TestResult:
    name: str
    status: TestStatus
    duration: float
    output: str
    error: Optional[str] = None

@dataclass
class ValidationReport:
    total_tests: int
    passed_tests: int
    failed_tests: int
    skipped_tests: int
    coverage: Optional[float] = None
    results: List[TestResult] = None

class TestValidator:
    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.output_dir = project_root / "test-results"
        self.results: List[TestResult] = []

    def validate_all(self) -> ValidationReport:
        """验证所有测试"""
        print("🔍 开始验证测试...")

        # 运行各类测试
        self._run_unit_tests()
        self._run_integration_tests()
        self._run_cli_tests()
        self._validate_test_coverage()
        self._check_test_quality()
        self._validate_mock_coverage()

        return self._generate_report()

    def _run_unit_tests(self) -> None:
        """运行单元测试"""
        print("📦 运行单元测试...")
        result = self._run_command(
            ["cargo", "test", "--lib", "--message-format=json"],
            "unit_tests"
        )
        self.results.append(result)

    def _run_integration_tests(self) -> None:
        """运行集成测试"""
        print("🔗 运行集成测试...")
        result = self._run_command(
            ["cargo", "test", "--test", "integration", "--message-format=json"],
            "integration_tests"
        )
        self.results.append(result)

    def _run_cli_tests(self) -> None:
        """运行CLI测试"""
        print("💻 运行CLI测试...")
        result = self._run_command(
            ["cargo", "test", "--test", "cli_integration", "--message-format=json"],
            "cli_tests"
        )
        self.results.append(result)

    def _run_command(self, cmd: List[str], test_name: str) -> TestResult:
        """运行命令并返回结果"""
        try:
            start_time = os.time.time()
            result = subprocess.run(
                cmd,
                cwd=self.project_root,
                capture_output=True,
                text=True,
                timeout=300
            )
            duration = os.time.time() - start_time

            # 解析JSON输出
            if result.returncode == 0:
                status = TestStatus.PASSED
                error = None
            else:
                status = TestStatus.FAILED
                error = result.stderr

            # 尝试解析JSON格式的测试输出
            test_count = 0
            passed_count = 0
            if result.stdout:
                try:
                    lines = result.stdout.strip().split('\n')
                    for line in lines:
                        if line.strip().startswith('{'):
                            data = json.loads(line)
                            if data.get('type') == 'test':
                                test_count += 1
                                if data.get('event') == 'passed':
                                    passed_count += 1
                except json.JSONDecodeError:
                    pass

            output = f"Tests: {passed_count}/{test_count} passed\n"
            output += f"Duration: {duration:.2f}s\n"
            if result.stdout:
                output += f"Output: {result.stdout[:500]}"

            return TestResult(
                name=test_name,
                status=status,
                duration=duration,
                output=output,
                error=error
            )

        except subprocess.TimeoutExpired:
            return TestResult(
                name=test_name,
                status=TestStatus.FAILED,
                duration=300.0,
                output="",
                error="Test timeout"
            )
        except Exception as e:
            return TestResult(
                name=test_name,
                status=TestStatus.FAILED,
                duration=0.0,
                output="",
                error=str(e)
            )

    def _validate_test_coverage(self) -> None:
        """验证测试覆盖率"""
        print("📊 验证测试覆盖率...")

        try:
            # 检查是否安装了cargo-llvm-cov
            result = subprocess.run(
                ["cargo", "llvm-cov", "--workspace", "--html", "--output-dir", "coverage"],
                cwd=self.project_root,
                capture_output=True,
                text=True,
                timeout=600
            )

            if result.returncode == 0:
                # 尝试提取覆盖率信息
                coverage = self._extract_coverage()
                print(f"✅ 代码覆盖率: {coverage:.1f}%")
            else:
                print("⚠️  覆盖率报告生成失败")

        except Exception as e:
            print(f"⚠️  覆盖率检查失败: {e}")

    def _extract_coverage(self) -> float:
        """从覆盖率报告中提取覆盖率百分比"""
        try:
            # 查找HTML覆盖率报告
            coverage_dir = self.project_root / "target" / "llvm-cov" / "html"
            index_file = coverage_dir / "index.html"

            if index_file.exists():
                content = index_file.read_text()
                # 简单的覆盖率提取（实际实现可能需要更复杂的解析）
                if "%" in content:
                    # 查找覆盖率百分比
                    import re
                    matches = re.findall(r'(\d+\.\d+)%', content)
                    if matches:
                        return float(matches[-1])  # 取最后一个匹配项

            return 0.0
        except Exception:
            return 0.0

    def _check_test_quality(self) -> None:
        """检查测试质量指标"""
        print("🔍 检查测试质量...")

        # 检查测试文件数量
        test_files = list(self.project_root.glob("**/*tests*.rs"))
        lib_tests = list(self.project_root.glob("**/tests/**/*.rs"))
        integration_tests = list(self.project_root.glob("tests/**/*.rs"))

        print(f"📁 测试文件统计:")
        print(f"  - 模块测试文件: {len(test_files)}")
        print(f"  - 库测试文件: {len(lib_tests)}")
        print(f"  - 集成测试文件: {len(integration_tests)}")

        # 检查测试命名规范
        self._check_test_naming(test_files + lib_tests + integration_tests)

        # 检查测试文档
        self._check_test_documentation(test_files + lib_tests + integration_tests)

    def _check_test_naming(self, test_files: List[Path]) -> None:
        """检查测试命名规范"""
        naming_issues = []

        for file_path in test_files:
            try:
                content = file_path.read_text(encoding='utf-8')
                lines = content.split('\n')

                for i, line in enumerate(lines, 1):
                    # 检查测试函数命名
                    if 'fn test_' in line or '#[test]' in line:
                        if 'fn test_' in line:
                            func_name = line.strip().split('fn test_')[1].split('(')[0]

                            # 检查命名规范（should_开头或描述性命名）
                            if not (func_name.startswith('should_') or
                                   func_name.startswith('test_') or
                                   '_' in func_name or
                                   len(func_name) >= 5):
                                naming_issues.append(f"{file_path}:{i} - {func_name}")

            except Exception:
                continue

        if naming_issues:
            print("⚠️  发现测试命名问题:")
            for issue in naming_issues[:10]:  # 只显示前10个
                print(f"  - {issue}")
            if len(naming_issues) > 10:
                print(f"  - ...还有 {len(naming_issues) - 10} 个问题")
        else:
            print("✅ 测试命名规范检查通过")

    def _check_test_documentation(self, test_files: List[Path]) -> None:
        """检查测试文档"""
        undocumented_tests = []

        for file_path in test_files:
            try:
                content = file_path.read_text(encoding='utf-8')
                lines = content.split('\n')

                in_test = False
                for i, line in enumerate(lines, 1):
                    if '#[test]' in line:
                        in_test = True
                        continue

                    if in_test and 'fn test_' in line:
                        # 检查测试函数前是否有文档注释
                        has_doc = False
                        for j in range(max(0, i-3), i):
                            if '///' in lines[j] or '/**' in lines[j]:
                                has_doc = True
                                break

                        if not has_doc:
                            func_name = line.strip().split('fn test_')[1].split('(')[0]
                            undocumented_tests.append(f"{file_path}:{i} - {func_name}")

                        in_test = False

            except Exception:
                continue

        if undocumented_tests:
            print("⚠️  发现未文档化的测试:")
            for test in undocumented_tests[:5]:  # 只显示前5个
                print(f"  - {test}")
            if len(undocumented_tests) > 5:
                print(f"  - ...还有 {len(undocumented_tests) - 5} 个未文档化的测试")
        else:
            print("✅ 测试文档检查通过")

    def _validate_mock_coverage(self) -> None:
        """验证Mock覆盖率"""
        print("🎭 验证Mock覆盖率...")

        # 检查是否有足够的Mock对象
        mock_files = list(self.project_root.glob("**/*mock*.rs"))
        test_util_files = list(self.project_root.glob("**/test_utils/**/*.rs"))

        print(f"📁 Mock文件统计:")
        print(f"  - Mock实现文件: {len(mock_files)}")
        print(f"  - 测试工具文件: {len(test_util_files)}")

        # 检查关键外部依赖是否被Mock
        critical_deps = [
            'google_drive', 'oauth', 'http', 'network', 'filesystem'
        ]

        for dep in critical_deps:
            mock_found = False
            for mock_file in mock_files + test_util_files:
                try:
                    content = mock_file.read_text(encoding='utf-8')
                    if dep.lower() in content.lower():
                        mock_found = True
                        break
                except Exception:
                    continue

            if mock_found:
                print(f"  ✅ {dep} - 已Mock")
            else:
                print(f"  ⚠️  {dep} - 缺少Mock")

    def _generate_report(self) -> ValidationReport:
        """生成验证报告"""
        total = len(self.results)
        passed = sum(1 for r in self.results if r.status == TestStatus.PASSED)
        failed = sum(1 for r in self.results if r.status == TestStatus.FAILED)
        skipped = sum(1 for r in self.results if r.status == TestStatus.SKIPPED)

        report = ValidationReport(
            total_tests=total,
            passed_tests=passed,
            failed_tests=failed,
            skipped_tests=skipped,
            results=self.results
        )

        # 打印总结
        print("\n" + "="*50)
        print("📊 测试验证总结")
        print("="*50)
        print(f"总测试套件: {total}")
        print(f"✅ 通过: {passed}")
        print(f"❌ 失败: {failed}")
        print(f"⏭️  跳过: {skipped}")

        if passed == total:
            print("🎉 所有测试验证通过!")
        else:
            print("💥 部分测试验证失败!")

        # 生成详细报告文件
        self._save_report_to_file(report)

        return report

    def _save_report_to_file(self, report: ValidationReport) -> None:
        """保存报告到文件"""
        report_file = self.output_dir / "validation_report.json"
        report_file.parent.mkdir(exist_ok=True)

        report_data = {
            "total_tests": report.total_tests,
            "passed_tests": report.passed_tests,
            "failed_tests": report.failed_tests,
            "skipped_tests": report.skipped_tests,
            "results": [
                {
                    "name": r.name,
                    "status": r.status.value,
                    "duration": r.duration,
                    "error": r.error
                }
                for r in report.results
            ]
        }

        with open(report_file, 'w', encoding='utf-8') as f:
            json.dump(report_data, f, indent=2, ensure_ascii=False)

        print(f"📄 详细报告已保存到: {report_file}")

def main():
    parser = argparse.ArgumentParser(description="Agentic-Warden 测试验证脚本")
    parser.add_argument(
        "--project-root",
        type=Path,
        default=Path.cwd(),
        help="项目根目录路径"
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="输出目录路径"
    )

    args = parser.parse_args()

    # 设置输出目录
    if args.output:
        output_dir = args.output
    else:
        output_dir = args.project_root / "test-results"

    # 创建验证器并运行
    validator = TestValidator(args.project_root)
    report = validator.validate_all()

    # 设置退出码
    sys.exit(0 if report.failed_tests == 0 else 1)

if __name__ == "__main__":
    main()