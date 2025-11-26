#!/usr/bin/env python3
"""
直接测试CODEX对JavaScript生成提示词的响应
"""

import subprocess
import sys
import os
import tempfile
import json
import time

def test_codex_js_generation():
    """测试CODEX生成JavaScript函数的能力"""
    print("🔧 测试CODEX JavaScript生成能力...")
    print("=" * 50)

    # 构建MCP工具注册JSON格式的提示词
    prompt = """Generate a complete MCP tool registration JSON for a JavaScript workflow that combines file operations.

## Workflow Plan:
- Step 1: Read a file using filesystem::read_file
- Step 2: Process and write content using filesystem::write_file

Requirements:
1. Generate a complete JSON object with name, description, input_schema, and js_code fields
2. The js_code must be exactly: async function workflow(input) with proper MCP calls
3. Use mcp.call("filesystem", "read_file", {...}) and mcp.call("filesystem", "write_file", {...})
4. Include try/catch error handling in the JavaScript code
5. The input_schema must define sourcePath and outputPath as required string properties

Expected JSON structure:
{
  "name": "file_processor_workflow",
  "description": "A workflow that reads a file and writes processed content to a new location",
  "input_schema": {
    "type": "object",
    "properties": {
      "sourcePath": { "type": "string", "description": "Path to source file" },
      "outputPath": { "type": "string", "description": "Path for output file" }
    },
    "required": ["sourcePath", "outputPath"]
  },
  "js_code": "async function workflow(input) { try { const fileResult = await mcp.call('filesystem', 'read_file', { path: input.sourcePath }); const content = typeof fileResult === 'string' ? fileResult : fileResult?.content ?? ''; const writeResult = await mcp.call('filesystem', 'write_file', { path: input.outputPath, content }); return { success: true, data: writeResult }; } catch (error) { return { success: false, error: error.message }; } }"
}

Respond with ONLY the JSON object, no markdown fences, no explanation."""

    print(f"📝 提示词长度: {len(prompt)} 字符")

    # 创建临时文件来传递提示词
    with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
        f.write(prompt)
        prompt_file = f.name

    try:
        # 构建CODEX命令
        codex_cmd = [
            "/home/putao/.nvm/versions/node/v24.5.0/bin/codex",
            "exec",
            "--dangerously-bypass-approvals-and-sandbox",
        ]

        print("🚀 调用CODEX生成JavaScript...")

        # 运行CODEX
        result = subprocess.run(
            codex_cmd,
            input=prompt + "\n",  # 直接通过stdin传递提示词
            text=True,
            capture_output=True,
            timeout=120,  # 2分钟超时
            cwd="/home/putao/code/rust/agentic-warden"
        )

        print(f"✅ CODEX调用完成，退出码: {result.returncode}")
        print(f"📄 输出长度: {len(result.stdout)} 字符")
        print(f"📄 错误长度: {len(result.stderr)} 字符")

        if result.returncode == 0 and result.stdout:
            print("\n🎯 CODEX生成的MCP工具JSON:")
            print("-" * 60)
            print(result.stdout)
            print("-" * 60)

            # 验证MCP工具JSON结构
            validation_results = validate_mcp_tool_json(result.stdout)
            print(f"\n📊 MCP工具JSON验证结果:")
            for check, passed in validation_results.items():
                status = "✅" if passed else "❌"
                print(f"{status} {check}")

            # 计算100%通过率
            passed_count = sum(1 for passed in validation_results.values() if passed)
            total_count = len(validation_results)
            pass_rate = (passed_count / total_count) * 100

            print(f"\n🎯 MCP工具JSON严格验证通过率: {pass_rate:.1f}% ({passed_count}/{total_count})")

            # 标记所有失败的检查项
            failed_checks = [check for check, passed in validation_results.items() if not passed]
            if failed_checks:
                print(f"❌ 失败的检查项: {', '.join(failed_checks)}")

            # 如果JSON验证通过，提取并显示js_code
            if validation_results.get("有效JSON格式", False) and validation_results.get("包含js_code字段", False):
                try:
                    data = json.loads(result.stdout)
                    js_code = data.get("js_code", "")
                    print(f"\n📄 提取的JavaScript代码 ({len(js_code)} 字符):")
                    print("┌" + "─" * 58 + "┐")
                    for i, line in enumerate(js_code.split('\n')):
                        print(f"│ {line:56s} │")
                    print("└" + "─" * 58 + "┘")
                except:
                    print("⚠️ 无法提取JavaScript代码进行展示")

            if pass_rate == 100.0:
                print("🎉 CODEX完全符合100%严格要求！生成了完整的MCP工具注册JSON！")
                return True
            else:
                print(f"⚠️ CODEX未达到100%要求 (需要100%, 实际: {pass_rate:.1f}%)")
                return False
        else:
            print("❌ CODEX调用失败")
            if result.stderr:
                print("错误信息:", result.stderr[:500])
            return False

    except subprocess.TimeoutExpired:
        print("⏱️ CODEX调用超时")
        return False
    except Exception as e:
        print(f"❌ 测试异常: {e}")
        return False
    finally:
        # 清理临时文件
        if os.path.exists(prompt_file):
            os.remove(prompt_file)

def validate_mcp_tool_json(json_str):
    """验证生成的MCP工具JSON结构 - 100%符合要求"""
    results = {}

    try:
        # 解析JSON
        data = json.loads(json_str)

        # 验证必须的字段
        required_fields = ["name", "description", "input_schema", "js_code"]
        for field in required_fields:
            results[f"包含{field}字段"] = field in data

        # 验证js_code内容
        if "js_code" in data:
            js_code = data["js_code"]
            js_validations = [
                ("async function workflow(input)", "精确的函数签名"),
                ("try {", "try块开始"),
                ("catch (", "catch块"),
                ("await mcp.call(", "MCP调用格式"),
                ("return { success: true", "成功返回格式"),
                ("return { success: false", "错误返回格式"),
                ("filesystem", "使用正确的server名称"),
                ("read_file", "使用正确的工具名称"),
                ("write_file", "使用正确的工具名称"),
            ]

            for pattern, description in js_validations:
                results[f"js_code包含{description}"] = pattern in js_code

            # JavaScript额外检查
            results["js_code无markdown"] = "```" not in js_code
            results["js_code函数完整性"] = js_code.count("{") == js_code.count("}")
            results["js_code正确错误处理"] = "catch (error)" in js_code and "error.message" in js_code

        # 验证input_schema结构
        if "input_schema" in data:
            schema = data["input_schema"]
            results["schema是object类型"] = schema.get("type") == "object"

            if "properties" in schema:
                props = schema["properties"]
                results["schema包含sourcePath"] = "sourcePath" in props
                results["schema包含outputPath"] = "outputPath" in props

                # 检查属性类型
                if "sourcePath" in props:
                    results["sourcePath是string类型"] = props["sourcePath"].get("type") == "string"
                if "outputPath" in props:
                    results["outputPath是string类型"] = props["outputPath"].get("type") == "string"

            if "required" in schema:
                required = schema["required"]
                results["required包含sourcePath"] = "sourcePath" in required
                results["required包含outputPath"] = "outputPath" in required

        # 验证name和description
        if "name" in data:
            name = data["name"]
            results["name不为空"] = len(name.strip()) > 0
            results["name是有效标识符"] = name.replace("_", "").replace("-", "").isalnum()

        if "description" in data:
            desc = data["description"]
            results["description不为空"] = len(desc.strip()) > 0

    except json.JSONDecodeError:
        results["有效JSON格式"] = False
        for key in list(results.keys()):
            results[key] = False

    return results

if __name__ == "__main__":
    success = test_codex_js_generation()
    print(f"\n🏁 最终结果: {'成功' if success else '失败'}")
    sys.exit(0 if success else 1)