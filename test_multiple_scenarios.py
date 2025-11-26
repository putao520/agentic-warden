#!/usr/bin/env python3
"""
测试多个场景的LLM编排和MCP工具注册
"""

import subprocess
import json
import sys
import os

def test_scenario(name, description, workflow_steps):
    """测试单个场景"""
    print(f"🧪 测试场景: {name}")
    print(f"📝 描述: {description}")
    print(f"🔧 步骤数: {len(workflow_steps)}")
    print("=" * 60)

    # 构建JSON格式的workflow steps
    steps_json = json.dumps(workflow_steps, indent=2)

    # 构建MCP工具JSON生成提示词
    prompt = f"""Generate a complete MCP tool registration JSON for this JavaScript workflow.

## Workflow Plan:
{steps_json}

Requirements:
1. Generate a complete JSON object with name, description, input_schema, and js_code fields
2. The js_code must be exactly: async function workflow(input) with proper MCP calls
3. Include try/catch error handling in the JavaScript code
4. The input_schema must define all required parameters based on the workflow steps
5. Use the exact tool names from the workflow plan

Expected JSON structure:
{{
  "name": "workflow_name",
  "description": "Workflow description",
  "input_schema": {{
    "type": "object",
    "properties": {{
      // Define properties based on workflow needs
    }},
    "required": ["param1", "param2"]
  }},
  "js_code": "async function workflow(input) {{ try {{ // MCP calls for each step }} catch (error) {{ return {{ success: false, error: error.message }}; }} }}"
}}

Respond with ONLY the JSON object, no markdown fences, no explanation."""

    print(f"📝 提示词长度: {len(prompt)} 字符")

    try:
        # 调用CODEX
        result = subprocess.run(
            ["/home/putao/.nvm/versions/node/v24.5.0/bin/codex", "exec", "--dangerously-bypass-approvals-and-sandbox"],
            input=prompt + "\n",
            text=True,
            capture_output=True,
            timeout=120,
            cwd="/home/putao/code/rust/agentic-warden"
        )

        if result.returncode == 0 and result.stdout:
            try:
                # 解析和验证JSON
                data = json.loads(result.stdout)

                print(f"✅ 成功生成: {data.get('name', 'unknown')}")
                print(f"📄 描述: {data.get('description', 'No description')}")

                # 验证关键字段
                validation = {
                    "包含name字段": "name" in data,
                    "包含description字段": "description" in data,
                    "包含input_schema字段": "input_schema" in data,
                    "包含js_code字段": "js_code" in data,
                    "js_code有效": "async function workflow" in data.get("js_code", ""),
                    "包含MCP调用": "mcp.call" in data.get("js_code", ""),
                    "包含错误处理": "try" in data.get("js_code", "") and "catch" in data.get("js_code", ""),
                }

                passed = sum(1 for v in validation.values())
                total = len(validation)

                print(f"📊 验证通过率: {passed}/{total} ({passed/total*100:.1f}%)")

                for check, status in validation.items():
                    symbol = "✅" if status else "❌"
                    print(f"  {symbol} {check}")

                return validation["包含js_code字段"] and validation["js_code有效"]

            except json.JSONDecodeError as e:
                print(f"❌ JSON解析失败: {e}")
                print(f"原始输出: {result.stdout[:200]}...")
                return False
        else:
            print(f"❌ CODEX调用失败")
            return False

    except subprocess.TimeoutExpired:
        print("⏱️ 超时")
        return False
    except Exception as e:
        print(f"❌ 异常: {e}")
        return False

def main():
    """运行多个场景测试"""
    print("🚀 测试多个LLM编排场景")
    print("=" * 80)

    scenarios = [
        {
            "name": "multi_file_processor",
            "description": "处理多个文件的数据聚合工作流",
            "steps": [
                {
                    "step": 1,
                    "tool": "filesystem::read_file",
                    "description": "读取第一个CSV文件",
                    "dependencies": []
                },
                {
                    "step": 2,
                    "tool": "filesystem::read_file",
                    "description": "读取第二个CSV文件",
                    "dependencies": []
                },
                {
                    "step": 3,
                    "tool": "data_processor::merge_csv",
                    "description": "合并两个CSV数据",
                    "dependencies": [1, 2]
                },
                {
                    "step": 4,
                    "tool": "filesystem::write_file",
                    "description": "保存合并后的结果",
                    "dependencies": [3]
                }
            ]
        },
        {
            "name": "api_chain_processor",
            "description": "API调用链和数据处理工作流",
            "steps": [
                {
                    "step": 1,
                    "tool": "http_client::get_data",
                    "description": "从API获取数据",
                    "dependencies": []
                },
                {
                    "step": 2,
                    "tool": "data_transformer::process_json",
                    "description": "转换和清洗JSON数据",
                    "dependencies": [1]
                },
                {
                    "step": 3,
                    "tool": "database::store_data",
                    "description": "存储处理后的数据",
                    "dependencies": [2]
                },
                {
                    "step": 4,
                    "tool": "notification::send_alert",
                    "description": "发送完成通知",
                    "dependencies": [3]
                }
            ]
        },
        {
            "name": "report_generator",
            "description": "报告生成和多格式输出工作流",
            "steps": [
                {
                    "step": 1,
                    "tool": "database::query_metrics",
                    "description": "查询性能指标数据",
                    "dependencies": []
                },
                {
                    "step": 2,
                    "tool": "data_analyzer::calculate_stats",
                    "description": "计算统计信息",
                    "dependencies": [1]
                },
                {
                    "step": 3,
                    "tool": "report_generator::create_html",
                    "description": "生成HTML报告",
                    "dependencies": [2]
                },
                {
                    "step": 4,
                    "tool": "report_generator::create_json",
                    "description": "生成JSON报告",
                    "dependencies": [2]
                },
                {
                    "step": 5,
                    "tool": "filesystem::write_file",
                    "description": "保存所有报告文件",
                    "dependencies": [3, 4]
                }
            ]
        }
    ]

    results = []
    for i, scenario in enumerate(scenarios, 1):
        print(f"\n{'='*20} 场景 {i}/{len(scenarios)} {'='*20}")
        success = test_scenario(scenario["name"], scenario["description"], scenario["steps"])
        results.append({
            "scenario": scenario["name"],
            "success": success,
            "steps": len(scenario["steps"])
        })
        print()

    # 总结结果
    print(f"\n🎯 测试总结")
    print("=" * 40)

    passed_count = sum(1 for r in results if r["success"])
    total_count = len(results)

    print(f"总场景数: {total_count}")
    print(f"成功场景数: {passed_count}")
    print(f"成功率: {passed_count/total_count*100:.1f}%")

    if passed_count == total_count:
        print("🎉 所有场景都成功生成MCP工具JSON！")
    else:
        print(f"⚠️ {total_count - passed_count} 个场景失败")

    for result in results:
        status = "✅" if result["success"] else "❌"
        print(f"  {status} {result['scenario']} ({result['steps']} steps)")

    return passed_count == total_count

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)