
import asyncio
import json
import sys
from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import Tool, TextContent

app = Server("dynamic-tools-server")

# 动态工具数据
DYNAMIC_TOOL = json.loads("""{"name": "document_analysis_workflow", "description": "Intelligent document analysis and reporting workflow", "input_schema": {"type": "object", "properties": {"documentPath": {"type": "string", "description": "Input document path"}, "outputPath": {"type": "string", "description": "Output report path"}, "analysisType": {"type": "string", "description": "Type of analysis to perform"}}, "required": ["documentPath", "outputPath", "analysisType"]}, "js_code": "async function workflow(input) {\n  try {\n    if (!input || typeof input.documentPath !== 'string' || typeof input.outputPath !== 'string' || typeof input.analysisType !== 'string') {\n      throw new Error('Invalid input: documentPath, outputPath, and analysisType are required strings.');\n    }\n\n    const readResult = await mcp.call('filesystem::read_document', {\n      path: input.documentPath\n    });\n\n    if (!readResult || !readResult.content) {\n      throw new Error('Failed to read document content.');\n    }\n\n    const extractResult = await mcp.call('nlp::extract_text', {\n      content: readResult.content\n    });\n\n    if (!extractResult || !extractResult.text) {\n      throw new Error('Failed to extract text from document.');\n    }\n\n    const analysisResult = await mcp.call('analytics::analyze_content', {\n      text: extractResult.text,\n      analysisType: input.analysisType\n    });\n\n    if (!analysisResult) {\n      throw new Error('Failed to analyze document content.');\n    }\n\n    const reportResult = await mcp.call('formatter::generate_report', {\n      analysis: analysisResult,\n      outputPath: input.outputPath\n    });\n\n    return {\n      success: true,\n      data: {\n        reportPath: (reportResult && reportResult.outputPath) ? reportResult.outputPath : input.outputPath,\n        analysisSummary: analysisResult.summary || null\n      }\n    };\n  } catch (error) {\n    return {\n      success: false,\n      error: error && error.message ? error.message : 'Unknown error',\n      details: error && error.stack ? error.stack : undefined\n    };\n  }\n}\n"}""")

@app.list_tools()
async def list_tools() -> list[Tool]:
    """列出所有可用工具，包括动态生成的工具"""
    tools = [
        Tool(
            name="echo",
            description="Echo the input text",
            inputSchema={
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to echo"}
                },
                "required": ["text"]
            }
        ),
        Tool(
            name=DYNAMIC_TOOL["name"],
            description=DYNAMIC_TOOL["description"],
            inputSchema=DYNAMIC_TOOL["input_schema"]
        )
    ]
    return tools

@app.call_tool()
async def call_tool(name: str, arguments: dict) -> list[TextContent]:
    """处理工具调用"""
    if name == "echo":
        return [TextContent(type="text", text=f"Echo: {arguments.get('text', '')}")]

    elif name == DYNAMIC_TOOL["name"]:
        # 执行动态生成的JavaScript代码
        js_code = DYNAMIC_TOOL["js_code"]

        # 创建模拟的MCP环境
        class MockMCP:
            async def call(self, server, tool, args):
                print(f"🔧 执行MCP调用: {server}::{tool}", file=sys.stderr)

                # 模拟不同工具的响应
                if "read_document" in tool:
                    return {"content": "Sample document content", "pages": 5}
                elif "extract_text" in tool:
                    return {"text": "Extracted text content", "words": 1000}
                elif "analyze_content" in tool:
                    return {"quality": 0.85, "readability": "Good"}
                elif "generate_report" in tool:
                    return {"report_path": arguments.get("outputPath", "report.pdf"), "status": "completed"}
                else:
                    return {"status": "unknown_tool"}

        # 创建全局mcp对象
        mcp = MockMCP()

        # 创建并执行JavaScript函数
        exec(f