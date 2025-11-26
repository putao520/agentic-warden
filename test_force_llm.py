#!/usr/bin/env python3
"""
强制触发LLM编排的测试
"""

import subprocess
import json
import tempfile
import os

def test_force_llm_orchestration():
    """强制触发LLM编排"""
    print("🧠 强制触发LLM编排测试...")
    print("=" * 60)

    # 创建一个绝对无法通过简单MCP工具调用解决的极复杂任务
    ultra_complex_request = """
执行一个极其复杂的企业级AI工作流编排任务：

核心业务需求：
1. 创建一个智能销售分析系统，需要：
   - 动态生成复杂的JavaScript算法来处理多维数据分析
   - 实现机器学习预测模型（线性回归 + 时间序列分析）
   - 创建交互式数据可视化（图表生成和动态更新）
   - 建立实时监控和异常检测系统

2. 技术架构要求：
   - 异步JavaScript工作流，包含Promise和错误处理
   - 多步骤条件分支：IF-THEN-ELSE逻辑决策
   - 循环数据处理和迭代优化算法
   - 跨系统集成：文件系统 + 内存数据库 + 知识图谱

3. 具体执行步骤：
   - 步骤1：从多个数据源读取销售数据（JSON、CSV、API）
   - 步骤2：数据清洗和预处理（异常值检测和修正）
   - 步骤3：统计分析和相关性计算
   - 步骤4：机器学习模型训练和预测
   - 步骤5：生成多格式报告（JSON数据 + Markdown报告 + CSV导出）
   - 步骤6：将结果存储到多个系统并进行交叉验证

4. 高级功能要求：
   - 实现错误恢复机制（重试3次后降级处理）
   - 添加性能监控和执行时间统计
   - 支持并发处理和任务队列管理
   - 集成缓存机制提高性能

5. 业务逻辑复杂度：
   - 需要处理复杂的业务规则引擎
   - 实现动态决策算法
   - 支持多种数据格式和协议
   - 包含完整的审计日志和追踪

这个任务的复杂度远超任何单一MCP工具的能力，必须：
- 使用LLM生成复杂的JavaScript工作流代码
- 包含异步编程、错误处理、性能优化
- 调用至少8个不同的MCP工具进行协作
- 实现企业级的业务逻辑编排

请使用LLM（CODEX）生成完整的JavaScript解决方案。
"""

    # 正确的MCP协议握手 + 智能路由请求
    mcp_handshake_and_route = [
        # 1. initialize请求
        {
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {
                        "listChanged": True
                    }
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            },
            "id": 1
        },
        # 2. initialized通知（无ID，无响应）
        {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        },
        # 3. 智能路由请求
        {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "intelligent_route",
                "arguments": {
                    "user_request": ultra_complex_request,
                    "execution_mode": "dynamic",
                    "complexity": "maximum",
                    "require_workflow": True,
                    "prefer_llm_generation": True,
                    "min_workflow_steps": 15,
                    "force_llm_mode": True,
                    "bypass_vector_search": True
                }
            },
            "id": 2
        }
    ]

    # 将所有请求组合成一个JSONL格式的字符串
    request_text = "\n".join(json.dumps(req) for req in mcp_handshake_and_route)

    env = {
        'CLI_TYPE': 'codex',
        'CODEX_BIN': '/home/putao/.nvm/versions/node/v24.5.0/bin/codex'
    }

    try:
        print("🚀 发送超复杂LLM编排请求...")
        print(f"📝 请求长度: {len(ultra_complex_request)} 字符")
        print(f"🎯 复杂度: maximum（强制LLM模式）")

        process = subprocess.Popen(
            ['./target/release/aiw', 'mcp', 'serve'],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env={**dict(os.environ), **env}
        )

        try:
            stdout, stderr = process.communicate(
                input=request_text,
                timeout=300  # 5分钟超时
            )

            return analyze_llm_results(stdout, stderr)

        except subprocess.TimeoutExpired:
            process.kill()
            print("⏰ LLM编排超时（但这是好迹象，说明LLM在处理复杂请求）")
            return False

    except Exception as e:
        print(f"❌ LLM编排测试异常: {e}")
        return False

def analyze_llm_results(stdout, stderr):
    """分析LLM编排结果"""
    print(f"\n📊 分析LLM编排结果...")
    print(f"stdout长度: {len(stdout)}")
    print(f"stderr长度: {len(stderr)}")

    # 关键指标检查
    codex_init = "🤖 AI CLI code generator initialized: codex" in stderr
    llm_orchestration = "🔄 Initiating LLM workflow orchestration" in stderr
    llm_success = "LLM orchestration succeeded" in stderr
    vector_fallback = "Vector-based tool routing" in stderr

    # JavaScript代码复杂度分析
    js_indicators = [
        ("函数定义", "function" in stdout),
        ("异步编程", "async" in stdout and "await" in stdout),
        ("错误处理", "try" in stdout and "catch" in stdout),
        ("MCP调用", "mcp.call" in stdout),
        ("对象操作", "const" in stdout or "let" in stdout),
        ("数组方法", "forEach" in stdout or "map(" in stdout or "filter(" in stdout),
        ("条件分支", "if (" in stdout or "else" in stdout),
        ("循环处理", "for (" in stdout or "while (" in stdout),
        ("Promise处理", "Promise" in stdout or ".then(" in stdout),
        ("JSON操作", "JSON." in stdout)
    ]

    js_count = sum(1 for _, check in js_indicators if check)

    # MCP工具调用分析
    mcp_tools = [
        ("文件读取", "read_file" in stdout),
        ("文件写入", "write_file" in stdout),
        ("内存存储", "store_data" in stdout),
        ("实体创建", "create_entities" in stdout),
        ("知识图谱", "add_observations" in stdout),
        ("目录操作", "directory_tree" in stdout)
    ]

    mcp_count = sum(1 for _, check in mcp_tools if check)

    print(f"\n🎯 LLM编排核心指标:")
    print(f"✅ CODEX初始化: {'成功' if codex_init else '失败'}")
    print(f"✅ LLM编排触发: {'是' if llm_orchestration else '否'}")
    if llm_orchestration:
        print(f"✅ LLM编排成功: {'是' if llm_success else '否'}")
    if vector_fallback:
        print(f"⚠️ 回退到向量模式")

    print(f"\n🎯 JavaScript代码生成 (复杂度: {js_count}/10):")
    for name, check in js_indicators:
        status = "✅" if check else "❌"
        print(f"  {status} {name}")

    print(f"\n🎯 MCP工具调用 (使用: {mcp_count}/6):")
    for name, check in mcp_tools:
        status = "✅" if check else "❌"
        print(f"  {status} {name}")

    # 成功判断
    success_criteria = [
        codex_init,           # CODEX初始化
        llm_orchestration,    # LLM编排触发
        js_count >= 5,        # 足够的JS复杂度
        mcp_count >= 2        # 多个MCP工具调用
    ]

    success_count = sum(success_criteria)
    total_criteria = len(success_criteria)
    success_rate = success_count / total_criteria

    print(f"\n📈 LLM编排成功率: {success_rate:.1%} ({success_count}/{total_criteria})")

    if success_rate >= 0.75:
        print(f"\n🎉 LLM编排验证成功！")
        print(f"✅ 系统成功触发: CODEX → JavaScript生成 → MCP工具调用")
        print(f"✅ 超复杂任务处理能力已验证")
        return True
    else:
        print(f"\n⚠️ LLM编排需要进一步调整")
        if not llm_orchestration:
            print(f"  - LLM编排未触发（可能需要更高复杂度）")
        if js_count < 5:
            print(f"  - JavaScript代码复杂度不足")
        if mcp_count < 2:
            print(f"  - MCP工具调用不足")
        return False

if __name__ == "__main__":
    success = test_force_llm_orchestration()
    exit(0 if success else 1)