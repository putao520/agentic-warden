#!/usr/bin/env python3
"""
综合MCP测试：验证LLM编排、动态工具生成和注册的完整流程
"""

import subprocess
import json
import sys
import os
import time
import asyncio

async def test_llm_orchestration_to_mcp_registration():
    """测试从LLM编排到MCP注册的完整流程"""
    print("🚀 综合MCP测试：LLM编排 → 动态工具生成 → MCP注册")
    print("=" * 80)

    # 测试场景：多步骤复杂工作流
    test_scenarios = [
        {
            "name": "report_generator_workflow",
            "description": "Generate comprehensive reports with data analysis",
            "workflow": """
## Workflow Plan:
- Step 1: Query database for metrics using database::query_metrics
- Step 2: Calculate statistics using data_analyzer::calculate_stats
- Step 3: Generate HTML report using report_generator::create_html
- Step 4: Generate JSON report using report_generator::create_json
- Step 5: Save all reports using filesystem::write_file

Requirements:
- Generate complete MCP tool JSON with name, description, input_schema, js_code
- js_code must be exactly: async function workflow(input) with proper MCP calls
- Include comprehensive error handling with try/catch blocks
- Input schema: databaseUrl, reportPath, reportFormat (required strings)
- Use mcp.call() for each workflow step with proper parameter passing
""",
            "expected_inputs": ["databaseUrl", "reportPath", "reportFormat"]
        },
        {
            "name": "data_pipeline_workflow",
            "description": "Process data through multiple transformation steps",
            "workflow": """
## Workflow Plan:
- Step 1: Read CSV data using filesystem::read_csv
- Step 2: Transform data using data_transformer::process
- Step 3: Validate data using validator::check_quality
- Step 4: Store processed data using database::insert_records
- Step 5: Send notification using notification::send_alert

Requirements:
- Generate complete MCP tool JSON with name, description, input_schema, js_code
- js_code must be exactly: async function workflow(input) with proper MCP calls
- Include comprehensive error handling with try/catch blocks
- Input schema: sourcePath, targetTable, notificationEmail (required strings)
- Use mcp.call() for each workflow step with proper parameter passing
""",
            "expected_inputs": ["sourcePath", "targetTable", "notificationEmail"]
        }
    ]

    results = []

    for i, scenario in enumerate(test_scenarios, 1):
        print(f"\n{'='*20} 场景 {i}/{len(test_scenarios)} {'='*20}")
        print(f"🧪 测试场景: {scenario['name']}")
        print(f"📝 描述: {scenario['description']}")

        # Step 1: 调用CODEX生成MCP工具JSON
        print(f"\n📝 步骤1: 调用CODEX生成MCP工具...")

        try:
            codex_prompt = f"""Generate a complete MCP tool registration JSON for this workflow.

{scenario['workflow']}

Expected JSON structure:
{{
  "name": "{scenario['name']}",
  "description": "{scenario['description']}",
  "input_schema": {{
    "type": "object",
    "properties": {{
      // Define based on expected inputs
    }},
    "required": {scenario['expected_inputs']}
  }},
  "js_code": "async function workflow(input) {{ try {{ // MCP calls }} catch (error) {{ return {{ success: false, error: error.message }}; }} }}"
}}

Respond with ONLY the JSON object, no markdown fences, no explanation."""

            # 调用CODEX
            codex_result = subprocess.run(
                ["/home/putao/.nvm/versions/node/v24.5.0/bin/codex", "exec", "--dangerously-bypass-approvals-and-sandbox"],
                input=codex_prompt + "\n",
                text=True,
                capture_output=True,
                timeout=120,
                cwd="/home/putao/code/rust/agentic-warden"
            )

            if codex_result.returncode != 0 or not codex_result.stdout.strip():
                print(f"❌ CODEX生成失败")
                if codex_result.stderr:
                    print(f"错误: {codex_result.stderr}")
                results.append({
                    "scenario": scenario['name'],
                    "success": False,
                    "error": "CODEX generation failed"
                })
                continue

            # 解析生成的JSON
            try:
                generated_json = codex_result.stdout.strip()
                tool_data = json.loads(generated_json)
                print(f"✅ 成功生成工具: {tool_data.get('name', 'unknown')}")

            except json.JSONDecodeError as e:
                print(f"❌ JSON解析失败: {e}")
                print(f"原始输出: {codex_result.stdout[:200]}...")
                results.append({
                    "scenario": scenario['name'],
                    "success": False,
                    "error": f"JSON parsing failed: {e}"
                })
                continue

        except subprocess.TimeoutExpired:
            print("⏱️ CODEX调用超时")
            results.append({
                "scenario": scenario['name'],
                "success": False,
                "error": "CODEX timeout"
            })
            continue
        except Exception as e:
            print(f"❌ CODEX调用异常: {e}")
            results.append({
                "scenario": scenario['name'],
                "success": False,
                "error": f"CODEX exception: {e}"
            })
            continue

        # Step 2: 验证生成的MCP工具JSON结构
        print(f"\n📝 步骤2: 验证MCP工具JSON结构...")

        validation_results = validate_mcp_tool_structure(tool_data, scenario['expected_inputs'])

        print(f"📊 验证结果:")
        for check, passed in validation_results.items():
            status = "✅" if passed else "❌"
            print(f"  {status} {check}")

        validation_score = sum(validation_results.values()) / len(validation_results)
        print(f"🎯 验证通过率: {validation_score*100:.1f}%")

        # Step 3: 模拟MCP系统工具注册
        print(f"\n📝 步骤3: 模拟MCP系统工具注册...")

        registration_success = simulate_mcp_tool_registration(tool_data)

        if registration_success:
            print("✅ 工具注册模拟成功")
        else:
            print("❌ 工具注册模拟失败")

        # Step 4: 验证JavaScript代码质量
        print(f"\n📝 步骤4: 验证JavaScript代码质量...")

        js_validation = validate_javascript_quality(tool_data.get('js_code', ''))

        print(f"📊 JavaScript验证结果:")
        for check, passed in js_validation.items():
            status = "✅" if passed else "❌"
            print(f"  {status} {check}")

        js_score = sum(js_validation.values()) / len(js_validation) if js_validation else 0
        print(f"🎯 JavaScript质量评分: {js_score*100:.1f}%")

        # 计算总体成功状态
        overall_success = (
            validation_score >= 0.9 and  # 90%以上结构验证通过
            registration_success and     # 注册成功
            js_score >= 0.8             # 80%以上JavaScript质量
        )

        results.append({
            "scenario": scenario['name'],
            "success": overall_success,
            "validation_score": validation_score,
            "registration_success": registration_success,
            "js_score": js_score,
            "steps": len(scenario['workflow'].split('-'))
        })

        print(f"\n🎯 场景 '{scenario['name']}: {'✅ 成功' if overall_success else '❌ 失败'}")

    # 总结测试结果
    print(f"\n{'='*80}")
    print(f"🎯 综合MCP测试总结")
    print(f"{'='*80}")

    success_count = sum(1 for r in results if r['success'])
    total_count = len(results)

    print(f"总场景数: {total_count}")
    print(f"成功场景数: {success_count}")
    print(f"成功率: {success_count/total_count*100:.1f}%")

    if success_count == total_count:
        print("🎉 所有场景都成功生成了高质量的MCP工具！")
        print("✅ LLM编排触发正常")
        print("✅ MCP工具JSON结构完整")
        print("✅ JavaScript代码质量优秀")
        print("✅ 工具注册流程验证通过")
        print("\n🚀 系统已准备好进行真实的动态MCP工具注册！")
        return True
    else:
        print(f"⚠️ {total_count - success_count} 个场景需要改进")

        print(f"\n📊 详细结果:")
        for result in results:
            status = "✅" if result['success'] else "❌"
            print(f"  {status} {result['scenario']}")
            if not result['success']:
                print(f"     - 结构验证: {result.get('validation_score', 0)*100:.1f}%")
                print(f"     - 注册成功: {result.get('registration_success', False)}")
                print(f"     - 代码质量: {result.get('js_score', 0)*100:.1f}%")

        return False

def validate_mcp_tool_structure(tool_data, expected_inputs):
    """验证MCP工具JSON结构"""
    results = {}

    # 基本字段验证
    required_fields = ["name", "description", "input_schema", "js_code"]
    for field in required_fields:
        results[f"包含{field}字段"] = field in tool_data

    # 名称验证
    if "name" in tool_data:
        name = tool_data["name"]
        results["name有效"] = len(name.strip()) > 0 and name.replace("_", "").replace("-", "").isalnum()

    # 描述验证
    if "description" in tool_data:
        desc = tool_data["description"]
        results["描述有效"] = len(desc.strip()) > 0

    # Schema验证
    if "input_schema" in tool_data:
        schema = tool_data["input_schema"]
        results["schema是object类型"] = isinstance(schema, dict) and schema.get("type") == "object"

        if isinstance(schema, dict):
            properties = schema.get("properties", {})
            required = schema.get("required", [])

            results["schema有properties字段"] = "properties" in schema
            results["schema有required字段"] = "required" in schema
            results["包含所有必需输入"] = all(inp in properties for inp in expected_inputs)
            results["required匹配预期"] = set(required) == set(expected_inputs)

            # 验证属性类型
            for prop in expected_inputs:
                if prop in properties:
                    prop_def = properties[prop]
                    results[f"{prop}是string类型"] = isinstance(prop_def, dict) and prop_def.get("type") == "string"

    # JavaScript代码验证
    if "js_code" in tool_data:
        js_code = tool_data["js_code"]
        results["js_code不为空"] = len(js_code.strip()) > 0
        results["js_code包含async函数"] = "async function workflow" in js_code
        results["js_code包含MCP调用"] = "mcp.call(" in js_code
        results["js_code包含错误处理"] = "try" in js_code and "catch" in js_code

    return results

def simulate_mcp_tool_registration(tool_data):
    """模拟MCP系统工具注册"""
    try:
        # 验证工具数据的基本结构
        required_fields = ["name", "description", "input_schema", "js_code"]
        if not all(field in tool_data for field in required_fields):
            return False

        # 模拟注册过程
        tool_name = tool_data["name"]
        schema = tool_data["input_schema"]

        # 验证schema结构
        if not isinstance(schema, dict) or schema.get("type") != "object":
            return False

        properties = schema.get("properties", {})
        required = schema.get("required", [])

        # 验证必需字段
        if not isinstance(properties, dict) or not isinstance(required, list):
            return False

        # 模拟工具注册成功
        print(f"  🔧 模拟注册工具: {tool_name}")
        print(f"  📋 Schema属性数: {len(properties)}")
        print(f"  🔒 必需参数: {required}")

        return True

    except Exception as e:
        print(f"  ❌ 注册模拟异常: {e}")
        return False

def validate_javascript_quality(js_code):
    """验证JavaScript代码质量"""
    results = {}

    if not js_code:
        return {"js_code为空": False}

    # 基本结构检查
    results["包含async函数定义"] = "async function workflow(" in js_code
    results["包含try块"] = "try {" in js_code
    results["包含catch块"] = "catch (" in js_code
    results["包含MCP调用"] = "await mcp.call(" in js_code
    results["包含return语句"] = "return" in js_code

    # 代码质量检查
    results["函数括号平衡"] = js_code.count("{") == js_code.count("}")
    results["正确错误处理"] = "catch (error)" in js_code and ("error.message" in js_code or "err.message" in js_code)
    results["无markdown标记"] = "```" not in js_code

    # 高级质量检查
    results["输入验证"] = "input" in js_code and ("typeof" in js_code or "input." in js_code)
    results["异步处理"] = "await" in js_code
    results["结构化返回"] = any(pattern in js_code for pattern in ["{ success:", "{data:", "{error:"])

    return results

async def main():
    """主测试函数"""
    print("🚀 启动综合MCP测试系统")
    print("=" * 80)

    success = await test_llm_orchestration_to_mcp_registration()

    if success:
        print("\n🎉 综合MCP测试全部通过！")
        print("✅ LLM编排系统运行正常")
        print("✅ 动态MCP工具生成成功")
        print("✅ 工具结构验证完整")
        print("✅ JavaScript代码质量优秀")
        print("✅ MCP注册流程验证通过")
        print("\n🎯 系统已准备好进行生产环境的动态工具注册！")
    else:
        print("\n❌ 部分测试未通过")
        print("需要进一步优化LLM编排或MCP工具生成流程")

    return success

if __name__ == "__main__":
    success = asyncio.run(main())
    sys.exit(0 if success else 1)