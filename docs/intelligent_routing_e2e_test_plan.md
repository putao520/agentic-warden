# Agentic-Warden 智能路由系统 E2E 测试方案

## 📋 测试目标

验证智能路由系统的两个分支流程完整性和代码生成质量：
1. **JavaScript工具路由流程** - LLM工作流规划 + JS代码生成 + MCP工具编排
2. **直接MCP调用流程** - 向量搜索 + 智能路由 + 直接工具调用

## 🏗️ 系统架构分析

### 两个分支流程

#### 分支1: JavaScript工具路由 (Workflow Orchestration)
```
用户请求 → LLM工作流规划 → JS代码生成 → Boa运行时执行 → MCP工具调用 → 结果返回
```

**核心组件**：
- `WorkflowPlannerEngine` - LLM驱动的工具规划
- `BoaRuntime` - JavaScript运行时
- `McpFunctionInjector` - MCP工具注入
- `SchemaValidator` - 模式验证

#### 分支2: 直接MCP调用 (Direct Routing)
```
用户请求 → 向量嵌入搜索 → 智能路由决策 → 直接MCP工具调用 → 结果返回
```

**核心组件**：
- `FastEmbedder` - 向量嵌入
- `MemRoutingIndex` - 内存索引
- `DecisionEngine` - 路由决策
- `McpConnectionPool` - 连接池

## 🧪 测试场景设计

### 场景1: 文件操作类任务
**目标**: 测试文件读取、写入、列表操作

**JavaScript路由测试**：
- 请求："读取/tmp/test.txt文件内容，并在末尾添加时间戳"
- 预期JS代码生成：文件读取 + 时间戳生成 + 文件写入
- 验证：生成代码质量、执行结果正确性

**直接路由测试**：
- 请求："读取文件内容"
- 预期：直接路由到filesystem工具
- 验证：工具选择准确性、执行结果

### 场景2: 数据处理类任务
**目标**: 测试JSON数据处理、转换

**JavaScript路由测试**：
- 请求："解析JSON文件，提取特定字段并生成报告"
- 预期JS代码生成：JSON解析 + 数据提取 + 报告生成
- 验证：复杂逻辑处理能力

**直接路由测试**：
- 请求："列出文件内容"
- 预期：直接调用filesystem工具
- 验证：简单操作路由

### 场景3: 多工具协作任务
**目标**: 测试跨MCP服务器的工具编排

**JavaScript路由测试**：
- 请求："从文件系统读取配置，通过memory工具存储，然后生成总结"
- 预期JS代码生成：多工具调用链
- 验证：工具编排正确性

**直接路由测试**：
- 请求："简单的文件操作"
- 预期：单工具直接调用
- 验证：单步操作效率

## 🔧 测试实现方案

### 1. 环境准备

#### OLLAMA环境
```bash
# 安装和配置OLLAMA
curl -fsSL https://ollama.com/install.sh | sh
ollama pull llama3.1:8b  # 用于代码生成
ollama pull llama3.1:70b  # 高质量代码生成（可选）
```

#### MCP服务器配置
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-filesystem", "/tmp"],
      "enabled": true
    },
    "memory": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-memory"],
      "enabled": true
    }
  }
}
```

### 2. 测试工具实现

#### Python测试框架
```python
class IntelligentRoutingTester:
    def __init__(self):
        self.project_root = Path(__file__).parent.parent
        self.aiw_binary = self.project_root / "target/release/aiw"

    def test_js_workflow_routing(self, user_request: str) -> TestResult:
        """测试JavaScript工作流路由"""
        pass

    def test_direct_mcp_routing(self, user_request: str) -> TestResult:
        """测试直接MCP路由"""
        pass

    def evaluate_code_quality(self, generated_code: str) -> QualityScore:
        """评估代码生成质量"""
        pass
```

#### 质量评估指标
```python
@dataclass
class QualityScore:
    syntax_correctness: float  # 语法正确性
    logic_correctness: float    # 逻辑正确性
    security_score: float      # 安全评分
    efficiency_score: float    # 效率评分
    maintainability: float     # 可维护性
```

### 3. 测试用例

#### 用例1: 文件处理任务
```python
def test_file_processing():
    """测试文件处理任务的智能路由"""

    # JavaScript路由测试
    js_test_cases = [
        {
            "request": "读取/tmp/data.json文件，提取所有用户的email地址，并保存到新文件",
            "expected_tools": ["read_file", "write_file"],
            "complexity": "medium"
        },
        {
            "request": "监控/tmp目录，当新文件出现时记录到memory中",
            "expected_tools": ["list_allowed_directories", "write_memory"],
            "complexity": "high"
        }
    ]

    # 直接路由测试
    direct_test_cases = [
        {
            "request": "读取文件内容",
            "expected_tool": "read_file",
            "complexity": "low"
        }
    ]
```

#### 用例2: OLLAMA代码生成质量测试
```python
def test_ollama_code_generation():
    """测试OLLAMA模式下的代码生成质量"""

    test_scenarios = [
        {
            "model": "llama3.1:8b",
            "request": "生成一个函数，计算斐波那契数列的第n项",
            "expected_quality": "acceptable"  # 可接受质量
        },
        {
            "model": "llama3.1:70b",
            "request": "实现一个HTTP服务器，支持文件上传和下载",
            "expected_quality": "high"  # 高质量
        }
    ]
```

### 4. 真实环境测试

#### 使用真实CODEX测试
```python
def test_with_real_codex():
    """使用真实CODEX测试代码生成"""

    if shutil.which("codex"):
        test_cases = [
            "创建一个REST API端点，处理用户认证",
            "实现数据缓存机制，支持TTL",
            "生成一个React组件，支持表格排序和分页"
        ]

        for case in test_cases:
            result = invoke_codex_with_routing(case)
            evaluate_real_code_quality(result)
```

#### OLLAMA本地测试
```python
def test_with_local_ollama():
    """使用本地OLLAMA测试"""

    ollama_models = ["llama3.1:8b", "llama3.1:70b"]

    for model in ollama_models:
        if ollama_model_available(model):
            test_code_generation_with_ollama(model)
```

## 📊 验证指标

### 1. 路由准确性
- **目标匹配度**: 选择工具与用户需求的匹配程度
- **路由效率**: 直接路由 vs JS路由的性能对比
- **决策准确性**: 智能路由决策的正确率

### 2. 代码生成质量
- **语法正确性**: 生成代码的语法正确率
- **功能正确性**: 代码实现功能的准确率
- **安全性**: 生成代码的安全性评分
- **效率**: 代码执行效率

### 3. 系统稳定性
- **并发处理**: 多个并发请求的处理能力
- **错误恢复**: 异常情况下的恢复能力
- **资源使用**: CPU和内存使用效率

## 🚀 实施计划

### 阶段1: 基础测试框架 (2天)
- [x] 分析系统架构
- [ ] 创建Python测试框架
- [ ] 配置测试环境

### 阶段2: 路由流程测试 (3天)
- [ ] 实现JavaScript路由测试
- [ ] 实现直接MCP路由测试
- [ ] 验证两个分支功能

### 阶段3: OLLAMA集成测试 (3天)
- [ ] 配置OLLAMA环境
- [ ] 实现代码生成质量评估
- [ ] 测试不同模型性能

### 阶段4: 真实AI CLI测试 (2天)
- [ ] 使用真实CODEX测试
- [ ] 性能基准测试
- [ ] 最终验证和报告

## 📝 测试报告模板

### 测试执行报告
```
# 智能路由系统E2E测试报告

## 执行概览
- 测试时间: 2025-XX-XX
- 测试环境: OLLAMA/CODEX
- 测试用例数: XX
- 通过率: XX%

## 路由准确性测试
- JavaScript路由准确率: XX%
- 直接MCP路由准确率: XX%
- 路由决策时间: XXms

## 代码生成质量测试
- 语法正确率: XX%
- 功能正确率: XX%
- 安全评分: XX/100
- 效率评分: XX/100

## 性能测试
- 平均响应时间: XXms
- 并发处理能力: XX req/s
- 资源使用率: XX%

## 问题发现
1. [问题描述]
2. [复现步骤]
3. [解决方案]
```

## 🔍 关键验证点

1. **分支选择逻辑**: 验证系统何时选择JS路由 vs 直接MCP路由
2. **工具发现能力**: 测试动态工具注册和发现机制
3. **代码生成能力**: 验证不同LLM模型下的代码生成质量
4. **错误处理**: 测试各种异常情况的处理
5. **性能表现**: 对比两个分支的执行效率

通过这个完整的测试方案，我们可以全面验证智能路由系统的功能和性能！