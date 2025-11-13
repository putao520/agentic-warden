//! Main memory manager that coordinates all memory functionality

use super::{
    config::MemoryConfig,
    embedding::EmbeddingService,
    semantic_search::SemanticSearch,
    todo_manager::{TodoItem, TodoManager},
    vector_store::{SearchResult, VectorStore},
};
use std::collections::HashMap;

pub struct MemoryManager {
    config: MemoryConfig,
    embedding_service: EmbeddingService,
    vector_store: VectorStore,
    semantic_search: SemanticSearch,
    todo_manager: TodoManager,
    is_initialized: bool,
}

impl MemoryManager {
    pub async fn new() -> anyhow::Result<Self> {
        let config = MemoryConfig::load_from_provider_config()?;
        config.validate()?;

        // Initialize services
        let embedding_service = EmbeddingService::new(&config.ollama_url, &config.embedding_model);
        let vector_store = VectorStore::new(&config.qdrant_url)?;

        // Initialize vector store collection
        vector_store.initialize_collection().await?;

        // Initialize search and todo manager
        let semantic_search = SemanticSearch::new(vector_store.clone(), embedding_service.clone());
        let todo_manager = TodoManager::new(semantic_search.clone());

        Ok(Self {
            config,
            embedding_service,
            vector_store,
            semantic_search,
            todo_manager,
            is_initialized: true,
        })
    }

    pub async fn test_connections(&self) -> anyhow::Result<HashMap<String, bool>> {
        let mut results = HashMap::new();

        // Test Ollama connection
        let ollama_ok = self
            .embedding_service
            .test_connection()
            .await
            .unwrap_or(false);
        results.insert("ollama".to_string(), ollama_ok);

        // Test Qdrant connection
        let qdrant_ok = self.vector_store.test_connection().await.unwrap_or(false);
        results.insert("qdrant".to_string(), qdrant_ok);

        Ok(results)
    }

    pub async fn store_conversation(
        &self,
        content: &str,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> anyhow::Result<String> {
        let mut final_metadata = metadata.unwrap_or_default();
        final_metadata.insert(
            "type".to_string(),
            serde_json::Value::String("conversation".to_string()),
        );
        final_metadata.insert(
            "timestamp".to_string(),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
        );

        self.semantic_search
            .add_memory(content, final_metadata)
            .await
    }

    pub async fn search_relevant_memories(
        &self,
        query: &str,
        limit: Option<u64>,
    ) -> anyhow::Result<Vec<SearchResult>> {
        self.semantic_search.search(query, limit, Some(0.5)).await
    }

    pub async fn extract_and_store_todos(&self, text: &str) -> anyhow::Result<Vec<TodoItem>> {
        let todos = self.todo_manager.extract_todos_from_text(text).await?;

        // Store each TODO in memory for searchability
        for todo in &todos {
            let mut metadata = HashMap::new();
            metadata.insert(
                "type".to_string(),
                serde_json::Value::String("todo".to_string()),
            );
            metadata.insert(
                "status".to_string(),
                serde_json::Value::String(format!("{:?}", todo.status)),
            );
            metadata.insert(
                "priority".to_string(),
                serde_json::Value::String(format!("{:?}", todo.priority)),
            );
            metadata.insert(
                "todo_id".to_string(),
                serde_json::Value::String(todo.id.clone()),
            );

            let content = format!("TODO: {}", todo.title);
            if let Some(desc) = &todo.description {
                let content_with_desc = format!("{}\n\n{}", content, desc);
                self.semantic_search
                    .add_memory(&content_with_desc, metadata.clone())
                    .await?;
            } else {
                self.semantic_search
                    .add_memory(&content, metadata.clone())
                    .await?;
            }
        }

        Ok(todos)
    }

    pub async fn create_todo(
        &self,
        title: &str,
        description: Option<String>,
        session_id: Option<String>,
    ) -> anyhow::Result<TodoItem> {
        self.todo_manager
            .create_todo(title, description, session_id)
            .await
    }

    pub async fn search_todos(
        &self,
        query: &str,
        status_filter: Option<super::todo_manager::TodoStatus>,
    ) -> anyhow::Result<Vec<TodoItem>> {
        self.todo_manager.search_todos(query, status_filter).await
    }

    pub async fn update_todo_status(
        &self,
        todo_id: &str,
        status: super::todo_manager::TodoStatus,
    ) -> anyhow::Result<()> {
        self.todo_manager.update_todo_status(todo_id, status).await
    }

    pub async fn get_todos_by_session_id(
        &self,
        session_id: &str,
        status_filter: Option<super::todo_manager::TodoStatus>,
    ) -> anyhow::Result<Vec<TodoItem>> {
        self.todo_manager
            .get_todos_by_session_id(session_id, status_filter)
            .await
    }

    pub fn get_config(&self) -> &MemoryConfig {
        &self.config
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    pub async fn get_memory_stats(&self) -> anyhow::Result<HashMap<String, serde_json::Value>> {
        let mut stats = HashMap::new();

        // Search for all memories
        let all_memories = self
            .semantic_search
            .search("", Some(1000), Some(0.0))
            .await?;

        let mut conversations = 0;
        let mut todos = 0;
        let mut others = 0;

        for result in &all_memories {
            if let Some(memory_type) = result.point.metadata.get("type").and_then(|v| v.as_str()) {
                match memory_type {
                    "conversation" => conversations += 1,
                    "todo" => todos += 1,
                    _ => others += 1,
                }
            }
        }

        stats.insert(
            "total_memories".to_string(),
            serde_json::Value::Number(serde_json::Number::from(all_memories.len())),
        );
        stats.insert(
            "conversations".to_string(),
            serde_json::Value::Number(serde_json::Number::from(conversations)),
        );
        stats.insert(
            "todos".to_string(),
            serde_json::Value::Number(serde_json::Number::from(todos)),
        );
        stats.insert(
            "others".to_string(),
            serde_json::Value::Number(serde_json::Number::from(others)),
        );

        Ok(stats)
    }
}
