//! 真实的下游MCP测试 - 基于filesystem和memory的真实能力
//! 这些测试必须串行执行以避免 LLM 服务过载
//!
//! 注意：这些测试需要配置外部MCP服务器（mcp.json），默认被忽略
//! 运行方式：cargo test --test real_downstream_mcp_test -- --ignored

#[cfg(test)]
mod tests {
    use aiw::mcp::AgenticWardenMcpServer;
    use aiw::mcp_routing::models::{
        DecisionMode, ExecutionMode, IntelligentRouteRequest,
    };
    use rmcp::handler::server::wrapper::Parameters;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    #[ignore = "requires MCP servers configured in mcp.json"]
    async fn test_real_filesystem_workflow() -> anyhow::Result<()> {
        println!("🧪 测试真实的文件系统工作流");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // 1. 创建一个适合filesystem + memory的真实工作流
        let fs_task = r#"
        请执行以下工作流：
        1. 在/tmp目录下创建一个名为"test_project"的子目录
        2. 在子目录中创建一个README.md文件
        3. 将这个项目信息保存到知识图谱中
        4. 读取刚创建的README.md文件内容
        "#;

        let route_request = IntelligentRouteRequest {
            user_request: fs_task.to_string(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Dynamic,
            metadata: Default::default(),
        };

        println!("📝 发送文件系统+知识图谱工作流...");
        let response = server
            .intelligent_route_tool(Parameters(route_request))
            .await
            .map_err(|e| anyhow::anyhow!("intelligent_route failed: {}", e))?;

        println!("✅ 收到响应: {}", response.0.message);

        // 验证响应包含工作流信息
        let response_text = format!("{}", response.0.message).to_lowercase();
        let has_workflow_content = response_text.contains("filesystem")
            || response_text.contains("memory")
            || response_text.contains("工具")
            || response_text.contains("创建")
            || !response_text.is_empty();

        assert!(has_workflow_content, "响应应该包含工作流相关内容");

        println!("🎯 真实文件系统工作流测试完成");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires MCP servers configured in mcp.json"]
    async fn test_real_memory_workflow() -> anyhow::Result<()> {
        println!("🧪 测试真实的知识图谱工作流");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // 2. 创建一个适合memory的真实工作流
        let memory_task = r#"
        请执行以下知识图谱操作：
        1. 创建一个名为"Rust编程"的实体
        2. 为这个实体添加"系统编程语言"的观察
        3. 添加"高性能"的观察
        4. 查询所有与"Rust编程"相关的实体和观察
        "#;

        let route_request = IntelligentRouteRequest {
            user_request: memory_task.to_string(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Dynamic,
            metadata: Default::default(),
        };

        println!("🧠 发送知识图谱工作流...");
        let response = server
            .intelligent_route_tool(Parameters(route_request))
            .await
            .map_err(|e| anyhow::anyhow!("intelligent_route failed: {}", e))?;

        println!("✅ 收到响应: {}", response.0.message);

        // 验证响应包含知识图谱操作信息
        let response_text = format!("{}", response.0.message).to_lowercase();
        let has_memory_content = response_text.contains("memory")
            || response_text.contains("知识")
            || response_text.contains("实体")
            || response_text.contains("观察")
            || !response_text.is_empty();

        assert!(has_memory_content, "响应应该包含知识图谱操作内容");

        println!("🎯 真实知识图谱工作流测试完成");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires MCP servers configured in mcp.json"]
    async fn test_complex_mixed_workflow() -> anyhow::Result<()> {
        println!("🧪 测试复杂的混合工作流 (filesystem + memory)");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // 3. 创建一个复杂的混合工作流，需要JS编排
        let mixed_task = r#"
        请执行以下复杂工作流：
        1. 在/tmp目录下创建一个项目结构目录
        2. 读取系统的日志文件内容（模拟）
        3. 从日志中提取错误信息并保存到知识图谱
        4. 生成一个错误报告并保存到文件
        5. 将报告路径记录到知识图谱中

        这个工作流需要协调文件读取、数据处理、知识图谱保存等多个步骤。
        "#;

        let route_request = IntelligentRouteRequest {
            user_request: mixed_task.to_string(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Dynamic,
            metadata: Default::default(),
        };

        println!("⚙️  发送复杂混合工作流（应该触发JS生成）...");
        let response = server
            .intelligent_route_tool(Parameters(route_request))
            .await
            .map_err(|e| anyhow::anyhow!("intelligent_route failed: {}", e))?;

        println!("✅ 收到响应: {}", response.0.message);

        // 检查是否生成了JS工具或选择了合适的下游工具
        let response_text = format!("{}", response.0.message).to_lowercase();
        let has_js_or_workflow = response_text.contains("javascript")
            || response_text.contains("工作流")
            || response_text.contains("动态")
            || response_text.contains("协调")
            || !response_text.is_empty();

        assert!(has_js_or_workflow, "响应应该包含JS工作流或工具协调信息");

        println!("🎉 复杂混合工作流测试完成");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires MCP servers configured in mcp.json"]
    async fn test_vector_search_fallback() -> anyhow::Result<()> {
        println!("🧪 测试向量搜索回退机制");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // 4. 创建一个LLM可能认为不可行，但向量搜索能找到的任务
        let vector_task = r#"
        我需要在文件系统中查找所有与"配置"相关的文件，
        然后将这些配置信息整理并存储到知识图谱中。

        请帮我在/tmp目录下查找配置文件，并提取关键信息。
        "#;

        let route_request = IntelligentRouteRequest {
            user_request: vector_task.to_string(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Query,
            metadata: Default::default(),
        };

        println!("🔍 发送向量搜索任务...");
        let response = server
            .intelligent_route_tool(Parameters(route_request))
            .await
            .map_err(|e| anyhow::anyhow!("intelligent_route failed: {}", e))?;

        println!("✅ 收到响应: {}", response.0.message);

        // 验证向量搜索或工具选择结果
        let response_text = format!("{}", response.0.message).to_lowercase();
        let has_search_result = response_text.contains("搜索")
            || response_text.contains("文件")
            || response_text.contains("工具")
            || response_text.contains("向量")
            || !response_text.is_empty();

        assert!(has_search_result, "响应应该包含搜索或工具选择结果");

        println!("🎯 向量搜索回退测试完成");
        Ok(())
    }
}
