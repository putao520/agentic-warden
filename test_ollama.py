#!/usr/bin/env python3
"""
OLLAMA代码生成质量测试
测试现有的qwen3:1.7b模型的代码生成能力
"""

import subprocess
import json
import time
import sys
import os

def test_ollama_available():
    """检查OLLAMA是否可用"""
    try:
        result = subprocess.run(['ollama', '--version'], capture_output=True, text=True, timeout=10)
        return result.returncode == 0
    except:
        return False

def test_ollama_models():
    """列出可用的OLLAMA模型"""
    try:
        result = subprocess.run(['ollama', 'list'], capture_output=True, text=True, timeout=10)
        if result.returncode == 0:
            lines = result.stdout.strip().split('\n')
            models = []
            for line in lines[1:]:  # 跳过标题行
                if line.strip():
                    parts = line.split()
                    if parts:
                        models.append(parts[0])
            return models
        return []
    except:
        return []

def test_ollama_code_generation(model, prompt):
    """测试OLLAMA模型的代码生成能力"""
    print(f"🧪 测试 {model} 模型...")
    print(f"请求: {prompt}")

    start_time = time.time()

    try:
        # 构造OLLAMA API请求
        ollama_request = {
            "model": model,
            "prompt": f"请生成Python代码来完成以下任务:\n\n{prompt}\n\n只返回代码，不要解释。",
            "stream": False
        }

        # 调用OLLAMA API
        result = subprocess.run(
            ['ollama', 'run', model, f"请生成Python代码来完成以下任务:\n\n{prompt}\n\n只返回代码，不要解释。"],
            capture_output=True,
            text=True,
            timeout=30
        )

        duration = time.time() - start_time

        if result.returncode == 0:
            generated_code = result.stdout.strip()
            return {
                "success": True,
                "code": generated_code,
                "duration": duration,
                "error": None
            }
        else:
            return {
                "success": False,
                "code": None,
                "duration": duration,
                "error": result.stderr.strip()
            }

    except subprocess.TimeoutExpired:
        return {
            "success": False,
            "code": None,
            "duration": 30,
            "error": "Timeout after 30 seconds"
        }
    except Exception as e:
        return {
            "success": False,
            "code": None,
            "duration": 0,
            "error": str(e)
        }

def evaluate_code_quality(code, expected_elements):
    """评估生成的代码质量"""
    if not code:
        return {
            "syntax_correct": False,
            "has_expected_elements": False,
            "quality_score": 0
        }

    score = 0

    # 检查基本语法
    syntax_indicators = ['def', 'return', ':', '    ']
    syntax_score = sum(1 for indicator in syntax_indicators if indicator in code) / len(syntax_indicators)
    if syntax_score > 0.5:
        score += 30

    # 检查预期的代码元素
    element_score = sum(1 for element in expected_elements if element.lower() in code.lower()) / len(expected_elements)
    if element_score > 0:
        score += 40 * element_score

    # 检查代码结构
    if 'def ' in code and 'return' in code:
        score += 20

    # 检查注释/文档
    if '#' in code or '"""' in code:
        score += 10

    return {
        "syntax_correct": syntax_score > 0.5,
        "has_expected_elements": element_score > 0,
        "quality_score": min(100, score)
    }

def main():
    """主测试函数"""
    print("🚀 OLLAMA代码生成质量测试")
    print("=" * 50)

    # 检查OLLAMA可用性
    if not test_ollama_available():
        print("❌ OLLAMA不可用")
        return False

    print("✅ OLLAMA可用")

    # 获取可用模型
    models = test_ollama_models()
    if not models:
        print("❌ 没有可用的OLLAMA模型")
        return False

    print(f"📋 可用模型: {', '.join(models)}")

    # 测试用例
    test_cases = [
        {
            "name": "斐波那契数列",
            "prompt": "生成一个Python函数，计算斐波那契数列的第n项",
            "expected_elements": ["fibonacci", "fib", "def", "return", "n"]
        },
        {
            "name": "文件读取",
            "prompt": "生成一个Python函数，读取文件内容并返回",
            "expected_elements": ["file", "read", "open", "def", "return"]
        },
        {
            "name": "HTTP请求",
            "prompt": "生成一个Python函数，发送HTTP GET请求",
            "expected_elements": ["http", "request", "get", "def", "return"]
        }
    ]

    results = []

    for model in models:
        if 'embed' in model.lower():
            print(f"\n⚠️ 跳过嵌入模型: {model}")
            continue

        print(f"\n🧪 测试模型: {model}")
        print("-" * 30)

        model_results = []

        for test_case in test_cases:
            print(f"\n📝 测试用例: {test_case['name']}")

            # 测试代码生成
            generation_result = test_ollama_code_generation(model, test_case['prompt'])

            if generation_result["success"]:
                # 评估代码质量
                quality = evaluate_code_quality(
                    generation_result["code"],
                    test_case["expected_elements"]
                )

                print(f"✅ 生成成功 ({generation_result['duration']:.2f}s)")
                print(f"📊 质量评分: {quality['quality_score']}/100")
                print(f"📝 代码长度: {len(generation_result['code'])} 字符")

                if len(generation_result["code"]) < 200:
                    print(f"💻 生成代码:\n{generation_result['code']}")
                else:
                    print(f"💻 代码预览: {generation_result['code'][:200]}...")

                model_results.append({
                    "test_case": test_case["name"],
                    "success": True,
                    "duration": generation_result["duration"],
                    "quality_score": quality["quality_score"],
                    "code_length": len(generation_result["code"])
                })
            else:
                print(f"❌ 生成失败: {generation_result['error']}")
                model_results.append({
                    "test_case": test_case["name"],
                    "success": False,
                    "error": generation_result["error"],
                    "duration": generation_result["duration"]
                })

        results.append({
            "model": model,
            "results": model_results
        })

    # 生成总结报告
    print("\n" + "=" * 50)
    print("📊 测试总结")
    print("=" * 50)

    for model_result in results:
        model = model_result["model"]
        model_results = model_result["results"]

        successful_tests = [r for r in model_results if r["success"]]
        failed_tests = [r for r in model_results if not r["success"]]

        print(f"\n🤖 模型: {model}")
        print(f"  ✅ 成功: {len(successful_tests)}/{len(model_results)}")
        print(f"  ❌ 失败: {len(failed_tests)}")

        if successful_tests:
            avg_duration = sum(r["duration"] for r in successful_tests) / len(successful_tests)
            avg_quality = sum(r["quality_score"] for r in successful_tests) / len(successful_tests)
            print(f"  ⏱️ 平均耗时: {avg_duration:.2f}s")
            print(f"  📈 平均质量: {avg_quality:.1f}/100")

        if failed_tests:
            print(f"  🚨 常见错误:")
            for failed in failed_tests:
                print(f"    - {failed.get('error', 'Unknown error')}")

    return True

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)