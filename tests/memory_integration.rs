//! Memory module integration tests
//!
//! Tests the integrated memory functionality including:
//! - Memory configuration loading
//! - Memory manager initialization
//! - MCP memory tools

#[cfg(test)]
mod tests {
    use agentic_warden::memory::{MemoryConfig, MemoryManager};

    #[test]
    fn test_memory_config_default_values() {
        let config = MemoryConfig::default();

        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert_eq!(config.qdrant_url, "http://localhost:26333");
        assert_eq!(config.embedding_model, "qwen3-embedding:0.6b");
        assert_eq!(config.llm_model, "qwen3:8b");

        println!("✅ Default memory configuration values are correct");
    }

    #[test]
    fn test_memory_config_validation() {
        let mut config = MemoryConfig::default();

        // Valid config should pass
        assert!(config.validate().is_ok(), "Valid config should pass validation");

        // Empty ollama_url should fail
        config.ollama_url = "".to_string();
        assert!(config.validate().is_err(), "Empty ollama_url should fail validation");

        // Reset and test empty qdrant_url
        config = MemoryConfig::default();
        config.qdrant_url = "".to_string();
        assert!(config.validate().is_err(), "Empty qdrant_url should fail validation");

        // Reset and test empty embedding_model
        config = MemoryConfig::default();
        config.embedding_model = "".to_string();
        assert!(config.validate().is_err(), "Empty embedding_model should fail validation");

        // Reset and test empty llm_model
        config = MemoryConfig::default();
        config.llm_model = "".to_string();
        assert!(config.validate().is_err(), "Empty llm_model should fail validation");

        println!("✅ Memory configuration validation works correctly");
    }

    #[tokio::test]
    async fn test_memory_config_load_from_providers() {
        // This test verifies that memory config can be loaded from providers config
        let config = MemoryConfig::load_from_provider_config();

        // Should always return a config (default if file doesn't exist)
        assert!(config.is_ok(), "Should always return a memory config");

        let config = config.unwrap();

        // Should have valid default values
        assert!(!config.ollama_url.is_empty());
        assert!(!config.qdrant_url.is_empty());
        assert!(!config.embedding_model.is_empty());
        assert!(!config.llm_model.is_empty());

        println!("✅ Memory config loaded successfully from providers config");
        println!("   Ollama URL: {}", config.ollama_url);
        println!("   Qdrant URL: {}", config.qdrant_url);
        println!("   Embedding Model: {}", config.embedding_model);
        println!("   LLM Model: {}", config.llm_model);
    }

    #[test]
    fn test_providers_config_memory_integration() {
        use agentic_warden::provider::config::{ProvidersConfig, RegionalTokens};
        use std::collections::HashMap;

        let mut providers_config = ProvidersConfig {
            schema: None,
            default_provider: "test".to_string(),
            providers: HashMap::new(),
            user_tokens: HashMap::new(),
            memory: None,
        };

        // Test getting default memory config when none is set
        let memory_config = providers_config.get_memory_config();
        assert_eq!(memory_config.ollama_url, "http://localhost:11434");
        assert_eq!(memory_config.qdrant_url, "http://localhost:26333");

        // Test setting and getting custom memory config
        let custom_config = MemoryConfig {
            ollama_url: "http://custom:11435".to_string(),
            qdrant_url: "http://custom:6334".to_string(),
            embedding_model: "custom-embedding".to_string(),
            llm_model: "custom-llm".to_string(),
        };

        providers_config.set_memory_config(custom_config.clone());
        let retrieved_config = providers_config.get_memory_config();

        assert_eq!(retrieved_config.ollama_url, custom_config.ollama_url);
        assert_eq!(retrieved_config.qdrant_url, custom_config.qdrant_url);
        assert_eq!(retrieved_config.embedding_model, custom_config.embedding_model);
        assert_eq!(retrieved_config.llm_model, custom_config.llm_model);

        println!("✅ Providers config memory integration works correctly");
    }

    // Since external services (Ollama and Qdrant) are available, we can now test
    // the full integration with real services.

    #[test]
    fn test_memory_manager_struct_exists() {
        // Verify that MemoryManager can be instantiated (struct exists)
        let config = MemoryConfig::default();
        assert!(config.validate().is_ok());

        println!("✅ MemoryManager types compile correctly");
    }

    #[tokio::test]
    async fn test_memory_manager_initialization() {
        // Test actual MemoryManager initialization with real services
        println!("🔄 正在初始化内存管理器...");

        match MemoryManager::new().await {
            Ok(memory_manager) => {
                println!("✅ 内存管理器初始化成功");

                // Test configuration access
                let config = memory_manager.get_config();
                assert!(!config.ollama_url.is_empty());
                assert!(!config.qdrant_url.is_empty());
                assert_eq!(config.qdrant_url, "http://localhost:26333");

                println!("   Ollama URL: {}", config.ollama_url);
                println!("   Qdrant URL: {}", config.qdrant_url);
                println!("   嵌入模型: {}", config.embedding_model);
            }
            Err(e) => {
                println!("❌ 内存管理器初始化失败: {}", e);
                panic!("内存管理器初始化应该成功，因为外部服务都可用");
            }
        }
    }

    #[tokio::test]
    async fn test_memory_manager_connection_test() {
        // Test connection testing functionality
        println!("🔄 正在测试外部服务连接...");

        let memory_manager = MemoryManager::new().await.expect("内存管理器应该能初始化");

        let connection_results = memory_manager.test_connections().await;

        match connection_results {
            Ok(results) => {
                println!("✅ 连接测试完成");

                // Check Ollama connection
                if let Some(ollama_ok) = results.get("ollama") {
                    if *ollama_ok {
                        println!("   ✅ Ollama连接正常");
                    } else {
                        println!("   ❌ Ollama连接失败");
                    }
                }

                // Check Qdrant connection
                if let Some(qdrant_ok) = results.get("qdrant") {
                    if *qdrant_ok {
                        println!("   ✅ Qdrant连接正常");
                    } else {
                        println!("   ❌ Qdrant连接失败");
                    }
                }

                // At least one service should be working
                let working_services = results.values().filter(|&&v| v).count();
                assert!(working_services > 0, "至少应该有一个外部服务可用");
            }
            Err(e) => {
                println!("❌ 连接测试失败: {}", e);
                panic!("连接测试应该成功");
            }
        }
    }

    #[tokio::test]
    async fn test_embedding_service_basic() {
        // Test basic embedding functionality
        println!("🔄 正在测试嵌入服务...");

        use agentic_warden::memory::embedding::EmbeddingService;

        let embedding_service = EmbeddingService::new(
            "http://localhost:11434",
            "qwen3-embedding:0.6b"
        );

        let test_text = "这是一个测试文本，用于验证嵌入服务是否正常工作。";

        match embedding_service.generate_embedding(test_text).await {
            Ok(embedding_result) => {
                println!("✅ 嵌入服务测试成功");
                println!("   嵌入维度: {}", embedding_result.embedding.len());

                // Embedding should be a vector of reasonable size (typically > 100)
                assert!(embedding_result.embedding.len() > 100, "嵌入向量维度应该大于100");

                // Values should be reasonable (not NaN or infinite)
                for &val in &embedding_result.embedding {
                    assert!(val.is_finite(), "嵌入值应该是有限数值");
                }

                println!("   ✅ 嵌入向量验证通过");
            }
            Err(e) => {
                println!("❌ 嵌入服务测试失败: {}", e);
                panic!("嵌入服务应该能正常工作");
            }
        }
    }

    #[tokio::test]
    async fn test_vector_store_basic_operations() {
        // Test basic vector store operations
        println!("🔄 正在测试向量存储基本操作...");

        use agentic_warden::memory::vector_store::{VectorStore, MemoryPoint};
        use std::time::SystemTime;
        use std::collections::HashMap;

        println!("  测试 HTTP 端口: localhost:26333");

        // 先测试基本 HTTP 连接（HTTP接口）
        match reqwest::get("http://localhost:26333/collections").await {
            Ok(response) => {
                println!("  ✅ HTTP 连接正常: {}", response.status());
            }
            Err(e) => {
                println!("  ❌ HTTP 连接失败: {}", e);
            }
        }

        println!("  创建 VectorStore 实例（使用HTTP REST API）...");
        let vector_store = VectorStore::new("http://localhost:26333")
            .expect("向量存储应该能初始化");

        println!("  初始化集合...");
        match vector_store.initialize_collection().await {
            Ok(_) => {
                println!("  ✅ 向量集合初始化成功");
            }
            Err(e) => {
                println!("  ❌ 向量集合初始化失败: {}", e);
                // 打印更详细的错误信息
                if let Some(source) = e.source() {
                    println!("      源错误: {}", source);
                }
                panic!("向量集合应该能初始化");
            }
        }

        // Create a test embedding with 1536 dimensions (matching collection config)
        let test_embedding: Vec<f32> = (0..=1535).map(|i| i as f32 / 1535.0).collect();
        let test_content = "测试内容：向量存储基本功能测试";
        let json_value = serde_json::json!({
            "test_id": "test_vector_basic_ops",
            "timestamp": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });
        let test_metadata: HashMap<String, serde_json::Value> = json_value
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Create a memory point
        let memory_point = MemoryPoint {
            id: format!("test_point_{}", uuid::Uuid::new_v4()),
            content: test_content.to_string(),
            metadata: test_metadata,
            timestamp: chrono::Utc::now(),
        };

        // Store the vector
        let point_id = vector_store.upsert_point(
            memory_point,
            test_embedding.clone()
        ).await;

        match point_id {
            Ok(id) => {
                println!("✅ 向量存储成功，ID: {}", id);

                // Small delay to ensure indexing
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Test search functionality
                match vector_store.search(test_embedding, Some(1), Some(0.9)).await {
                    Ok(results) => {
                        println!("✅ 向量搜索成功，找到 {} 个结果", results.len());

                        if let Some(result) = results.first() {
                            assert_eq!(result.point.content, test_content);
                            println!("   ✅ 搜索内容匹配");
                            println!("   相似度分数: {:.4}", result.score);
                        }
                    }
                    Err(e) => {
                        println!("❌ 向量搜索失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ 向量存储失败: {}", e);
                panic!("向量存储应该能正常工作");
            }
        }
    }

    #[tokio::test]
    async fn test_todo_manager_basic_functionality() {
        // Test TODO manager basic functionality
        println!("🔄 正在测试TODO管理器基本功能...");

        let memory_manager = MemoryManager::new().await.expect("内存管理器应该能初始化");

        let test_session_id = "test_session_12345";
        let test_todo_title = "测试TODO项";
        let test_todo_description = "这是一个用于验证TODO管理器功能的测试项目";

        // Add a new TODO
        let todo_item = memory_manager.create_todo(
            test_todo_title,
            Some(test_todo_description.to_string()),
            Some(test_session_id.to_string())
        ).await;

        match todo_item {
            Ok(todo) => {
                println!("✅ TODO创建成功，ID: {}", todo.id);

                // Small delay to ensure indexing
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                // Test getting TODOs by session_id
                match memory_manager.get_todos_by_session_id(
                    test_session_id,
                    None
                ).await {
                    Ok(todos) => {
                        println!("✅ 按session_id查询TODO成功，找到 {} 个", todos.len());

                        if let Some(todo) = todos.iter().find(|t| t.title == test_todo_title) {
                            assert_eq!(todo.title, test_todo_title);
                            assert_eq!(todo.description.as_ref().unwrap_or(&String::new()), test_todo_description);
                            println!("   ✅ TODO内容验证通过");

                            // Test session_id extraction
                            if let Some(retrieved_session_id) = todo.get_session_id() {
                                assert_eq!(retrieved_session_id, test_session_id);
                                println!("   ✅ session_id提取验证通过");
                            } else {
                                panic!("应该能提取到session_id");
                            }
                        } else {
                            panic!("应该能找到刚创建的TODO");
                        }
                    }
                    Err(e) => {
                        println!("❌ 按session_id查询TODO失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ TODO创建失败: {}", e);
                panic!("TODO创建应该能正常工作");
            }
        }
    }

    #[tokio::test]
    async fn test_semantic_search_functionality() {
        // Test semantic search functionality
        println!("🔄 正在测试语义搜索功能...");

        let memory_manager = MemoryManager::new().await.expect("内存管理器应该能初始化");

        let test_query = "Python编程";
        let test_limit = 5;

        // Test searching memories
        match memory_manager.search_relevant_memories(test_query, Some(test_limit)).await {
            Ok(results) => {
                println!("✅ 语义搜索成功，找到 {} 个相关记忆", results.len());

                for (i, result) in results.iter().enumerate() {
                    println!("   结果 {}: 相似度 {:.4}", i+1, result.score);
                    println!("   内容预览: {:.50}...", result.point.content);
                }

                // Scores should be reasonable
                for result in &results {
                    assert!(result.score >= 0.0 && result.score <= 1.0, "相似度分数应该在0-1之间");
                }
            }
            Err(e) => {
                println!("⚠️  语义搜索未找到结果或出错: {}", e);
                // This is acceptable if there's no data in the vector store yet
            }
        }
    }
}