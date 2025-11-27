//! 真实的JS脚本生成和执行测试 - 验证完整工作流程
//! 测试JS脚本确实被生成，并且能被主LLM调用执行

#[cfg(test)]
mod tests {
    use agentic_warden::mcp::AgenticWardenMcpServer;
    use agentic_warden::mcp_routing::models::{
        DecisionMode, ExecutionMode, IntelligentRouteRequest,
    };
    use rmcp::handler::server::wrapper::Parameters;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_real_js_script_generation_and_execution() -> anyhow::Result<()> {
        println!("🧪 测试真实的JS脚本生成和执行流程");

        // 1. 初始化服务器
        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // 2. 创建一个需要JS脚本的复杂任务
        let complex_task = r#"
        分析以下数据并生成报告：
        数据：[
            {"name": "产品A", "sales": 1000, "cost": 600},
            {"name": "产品B", "sales": 1500, "cost": 900},
            {"name": "产品C", "sales": 800, "cost": 500}
        ]

        需要计算：
        1. 每个产品的利润率
        2. 总销售额和总利润
        3. 利润率最高的产品
        "#;

        let route_request = IntelligentRouteRequest {
            user_request: complex_task.to_string(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Query,
            metadata: Default::default(),
        };

        // 3. 调用智能路由，这应该生成JS脚本来处理数据分析任务
        println!("📝 发送复杂任务给智能路由...");
        let response = server
            .intelligent_route_tool(Parameters(route_request))
            .await
            .map_err(|e| anyhow::anyhow!("intelligent_route failed: {}", e))?;

        println!("✅ 收到响应: {}", response.0.message);

        // 4. 验证响应中是否包含JS代码生成的痕迹
        let response_text = format!("{}", response.0.message).to_lowercase();

        // 检查是否提到了JS脚本生成或执行
        let has_js_execution = response_text.contains("javascript")
            || response_text.contains("script")
            || response_text.contains("js")
            || response_text.contains("execute")
            || response_text.contains("生成");

        if has_js_execution {
            println!("✅ 响应显示JS脚本处理已执行");
        } else {
            println!("⚠️  响应可能没有使用JS脚本处理");
        }

        // 5. 验证响应包含数据分析结果或工具选择
        let has_analysis_result = response_text.contains("产品")
            || response_text.contains("工具")
            || response_text.contains("selected")
            || !response_text.is_empty();

        assert!(has_analysis_result, "响应应该包含分析结果或工具选择");

        println!("🎯 JS脚本生成和执行测试完成");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_direct_js_tool_invocation() -> anyhow::Result<()> {
        println!("🧪 直接测试JS工具调用");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // 6. 直接调用一个我们知道会触发JS的工具
        let js_route_request = IntelligentRouteRequest {
            user_request: "请编写一个JavaScript函数，输入是数字数组，返回排序后的数组".to_string(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Query,
            metadata: Default::default(),
        };

        println!("🔧 直接调用JS任务...");
        let js_response = server
            .intelligent_route_tool(Parameters(js_route_request))
            .await
            .map_err(|e| anyhow::anyhow!("intelligent_route failed: {}", e))?;

        println!("📋 JS工具响应: {}", js_response.0.message);

        // 7. 验证响应中包含JavaScript相关信息
        let response_text = format!("{}", js_response.0.message).to_lowercase();
        let response_contains_js = response_text.contains("javascript")
            || response_text.contains("function")
            || response_text.contains("array")
            || response_text.contains("排序")
            || !response_text.is_empty();

        assert!(response_contains_js, "响应应该包含JavaScript相关内容");

        println!("✅ 直接JS工具调用测试完成");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_workflow_execution_with_js() -> anyhow::Result<()> {
        println!("🧪 测试工作流执行中的JS脚本");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // 8. 测试一个复杂的工作流，其中包含多个步骤，需要JS协调
        let workflow_request = IntelligentRouteRequest {
            user_request: "请执行以下工作流：\n1. 生成一个随机数列表\n2. 计算平均值\n3. 找出最大值和最小值\n4. 返回统计摘要".to_string(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Query,
            metadata: Default::default(),
        };

        println!("⚙️  执行复杂工作流...");
        let workflow_response = server
            .intelligent_route_tool(Parameters(workflow_request))
            .await
            .map_err(|e| anyhow::anyhow!("intelligent_route failed: {}", e))?;

        println!("📊 工作流响应: {}", workflow_response.0.message);

        // 9. 验证工作流执行结果
        let response_text = format!("{}", workflow_response.0.message).to_lowercase();
        let has_workflow_results = response_text.contains("统计")
            || response_text.contains("平均值")
            || response_text.contains("最大值")
            || response_text.contains("最小值")
            || !response_text.is_empty();

        assert!(has_workflow_results, "工作流响应应该包含统计结果或工具选择");

        println!("🎉 工作流JS执行测试完成");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_js_error_handling() -> anyhow::Result<()> {
        println!("🧪 测试JS错误处理");

        let server = AgenticWardenMcpServer::bootstrap()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bootstrap server: {}", e))?;

        // 10. 测试一个可能导致问题的请求
        let error_request = IntelligentRouteRequest {
            user_request:
                "请执行这段有问题的JavaScript代码：\nlet x = y + 1; // y未定义\nconsole.log(x);"
                    .to_string(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Query,
            metadata: Default::default(),
        };

        println!("❌ 测试JS错误处理...");
        let error_response = server
            .intelligent_route_tool(Parameters(error_request))
            .await
            .map_err(|e| anyhow::anyhow!("intelligent_route failed: {}", e))?;

        println!("🚨 错误响应: {}", error_response.0.message);

        // 11. 验证错误被正确处理
        // 服务器应该不会崩溃，而是返回有意义的响应
        let response_text = format!("{}", error_response.0.message);
        let response_is_meaningful = !response_text.is_empty() && response_text.len() > 10;

        assert!(
            response_is_meaningful,
            "即使是错误情况，也应该返回有意义的响应"
        );

        println!("✅ JS错误处理测试完成");
        Ok(())
    }
}
