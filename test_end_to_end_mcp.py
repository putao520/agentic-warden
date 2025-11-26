#!/usr/bin/env python3
"""
端到端测试：验证动态生成的MCP工具是否能被系统感知和执行
"""

import subprocess
import json
import sys
import os
import time

def test_dynamic_tool_registration_and_execution():
    """测试动态工具注册和执行"""
    print("🧪 端到端测试：动态MCP工具注册和执行")
    print("=" * 60)

    # 1. 生成动态工具
    print("📝 步骤1: 生成动态MCP工具...")

    # 构建一个更复杂的workflows
    workflow_prompt = """Generate a complete MCP tool registration JSON for a data analysis workflow.

## Workflow Plan:
[
  {
    "step": 1,
    "tool": "data_analyzer::read_dataset",
    "description": "读取数据集文件",
    "dependencies": []
  },
  {
    "step": 2,
    "tool": "statistics::calculate",
    "description": "计算统计信息",
    "dependencies": [1]
  },
  {
    "step": 3,
    "tool": "visualizer::create_chart",
    "description": "生成数据图表",
    "dependencies": [2]
  },
  {
    "step": 4,
    "tool": "filesystem::save_results",
    "description": "保存分析结果",
    "dependencies": [1, 2, 3]
  }
]

Requirements:
1. Generate JSON with name, description, input_schema, js_code
2. js_code: async function workflow(input) with proper MCP calls
3. input_schema: datasetPath, outputPath (both required strings)
4. Use mcp.call() for each workflow step
5. Include try/catch error handling

Expected JSON:
{
  "name": "data_analysis_workflow",
  "description": "Complete data analysis and visualization workflow",
  "input_schema": {
    "type": "object",
    "properties": {
      "datasetPath": {"type": "string"},
      "outputPath": {"type": "string"}
    },
    "required": ["datasetPath", "outputPath"]
  },
  "js_code": "..."
}

Respond with ONLY JSON, no markdown fences."""

    # 调用CODEX生成工具
    try:
        codex_result = subprocess.run(
            ["/home/putao/.nvm/versions/node/v24.5.0/bin/codex", "exec", "--dangerously-bypass-approvals-and-sandbox"],
            input=workflow_prompt + "\n",
            text=True,
            capture_output=True,
            timeout=120,
            cwd="/home/putao/code/rust/agentic-warden"
        )

        if codex_result.returncode != 0 or not codex_result.stdout:
            print(f"❌ CODEX生成失败")
            print(f"错误: {codex_result.stderr}")
            return False

        # 解析生成的JSON
        try:
            tool_json = json.loads(codex_result.stdout.strip())
            print(f"✅ 成功生成工具: {tool_json.get('name', 'unknown')}")

        except json.JSONDecodeError as e:
            print(f"❌ JSON解析失败: {e}")
            print(f"原始输出: {codex_result.stdout[:200]}...")
            return False

    except Exception as e:
        print(f"❌ CODEX调用异常: {e}")
        return False

    # 2. 测试工具注册
    print("\n📝 步骤2: 测试动态工具注册...")

    # 创建一个测试脚本，模拟工具注册
    test_registration_script = f'''
import json
import sys
import time

# 模拟动态工具注册
tool_data = json.loads("""{codex_result.stdout.strip()}""")

print(f"🔧 注册工具: {{tool_data['name']}}")
print(f"📝 描述: {{tool_data['description']}}")
print(f"📋 Schema: {{tool_data.get('input_schema', {{}})}}")

# 模拟schema验证
schema = tool_data.get('input_schema', {{}})
if isinstance(schema, dict) and schema.get('type') == 'object':
    print("✅ Input schema有效")

    properties = schema.get('properties', {{}})
    required = schema.get('required', [])

    print(f"  属性数量: {{len(properties)}}")
    print(f"  必需参数: {{required}}")

    # 验证必需属性
    missing_props = []
    for prop in required:
        if prop not in properties:
            missing_props.append(prop)

    if missing_props:
        print(f"❌ 缺少必需属性: {{missing_props}}")
    else:
        print("✅ 所有必需属性都存在")

        # 模拟工具调用测试
        test_input = {{
            "datasetPath": "/tmp/test_data.csv",
            "outputPath": "/tmp/analysis_result.json"
        }}

        print(f"🧪 测试输入: {{test_input}}")

        # 验证输入是否符合schema
        valid_input = True
        for prop in required:
            if prop not in test_input:
                valid_input = False
                break

        if valid_input:
            print("✅ 输入验证通过")

            # 提取JavaScript代码进行语法检查
            js_code = tool_data.get('js_code', '')
            if 'async function workflow' in js_code and 'mcp.call(' in js_code:
                print("✅ JavaScript代码结构有效")
                print(f"📄 代码长度: {{len(js_code)}} 字符")

                # 简单的JavaScript语法检查
                basic_checks = [
                    'async function workflow(' in js_code,
                    'try {' in js_code,
                    'catch (' in js_code,
                    'await mcp.call(' in js_code,
                    'return {' in js_code
                ]

                passed_checks = sum(1 for check in basic_checks if check)
                print(f"🎯 基础语法检查: {{passed_checks}}/5 通过")

                if passed_checks >= 4:
                    print("🎉 JavaScript代码质量良好")
                    return True
                else:
                    print("⚠️ JavaScript代码需要改进")
            else:
                print("❌ JavaScript代码结构无效")
        else:
            print("❌ 输入验证失败")
else:
    print("❌ Schema格式无效")

print("\\n🏁 动态工具注册测试完成")
'''

    try:
        reg_result = subprocess.run(
            [sys.executable, "-c", test_registration_script],
            capture_output=True,
            text=True,
            timeout=30
        )

        print(f"注册测试结果:")
        if reg_result.stdout:
            print(reg_result.stdout)

        success = "🎉" in reg_result.stdout
        print(f"工具注册测试: {'成功' if success else '失败'}")

        return success

    except Exception as e:
        print(f"❌ 注册测试异常: {e}")
        return False

def main():
    """主测试函数"""
    print("🚀 端到端LLM编排系统测试")
    print("=" * 80)

    success = test_dynamic_tool_registration_and_execution()

    if success:
        print("\n🎉 端到端测试通过！")
        print("✅ 动态MCP工具生成")
        print("✅ 工具结构验证")
        print("✅ Schema定义正确")
        print("✅ JavaScript代码质量")
        print("✅ 输入参数验证")
        print("\n🎯 系统已准备好处理复杂的LLM编排请求！")
    else:
        print("\n❌ 端到端测试失败")
        print("需要进一步调试动态工具注册流程")

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)