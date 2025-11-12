//! TODO management functionality

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TodoPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl TodoItem {
    // Helper methods to extract session_id from metadata
    pub fn get_session_id(&self) -> Option<String> {
        self.metadata.get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn set_session_id(&mut self, session_id: String) {
        self.metadata.insert("session_id".to_string(), serde_json::Value::String(session_id));
    }
}

pub struct TodoManager {
    storage: super::semantic_search::SemanticSearch,
}

impl TodoManager {
    pub fn new(storage: super::semantic_search::SemanticSearch) -> Self {
        Self { storage }
    }

    pub async fn extract_todos_from_text(&self, text: &str) -> anyhow::Result<Vec<TodoItem>> {
        let mut todos = Vec::new();

        // Simple TODO extraction patterns
        let patterns = [
            r"(?i)\bTODO\b:\s*(.+)",
            r"(?i)\bFIX\b:\s*(.+)",
            r"(?i)\bNOTE\b:\s*(.+)",
            r"(?i)\bBUG\b:\s*(.+)",
            r"(?i)\bIMPROVE\b:\s*(.+)",
            r"(?i)\bREFACTOR\b:\s*(.+)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for caps in re.captures_iter(text) {
                    if let Some(todo_text) = caps.get(1) {
                        let todo = TodoItem {
                            id: uuid::Uuid::new_v4().to_string(),
                            title: todo_text.as_str().trim().to_string(),
                            description: None,
                            status: TodoStatus::Pending,
                            priority: TodoPriority::Medium,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                            tags: vec![self.detect_todo_type(&todo_text.as_str().to_lowercase())],
                            metadata: HashMap::new(),
                        };
                        todos.push(todo);
                    }
                }
            }
        }

        // Sort by priority
        todos.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(todos)
    }

    pub async fn create_todo(&self, title: &str, description: Option<String>, session_id: Option<String>) -> anyhow::Result<TodoItem> {
        let mut metadata = HashMap::new();
        if let Some(session_id) = session_id {
            metadata.insert("session_id".to_string(), serde_json::Value::String(session_id));
        }

        let todo = TodoItem {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            description,
            status: TodoStatus::Pending,
            priority: TodoPriority::Medium,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec![],
            metadata,
        };

        // Store as memory for searchability
        let mut metadata = todo.metadata.clone();
        metadata.insert("type".to_string(), serde_json::Value::String("todo".to_string()));
        metadata.insert("status".to_string(), serde_json::Value::String(format!("{:?}", todo.status)));
        metadata.insert("priority".to_string(), serde_json::Value::String(format!("{:?}", todo.priority)));

        let content = format!("TODO: {}", todo.title);
        if let Some(desc) = &todo.description {
            let content_with_desc = format!("{}\n\n{}", content, desc);
            self.storage.add_memory(&content_with_desc, metadata).await?;
        } else {
            self.storage.add_memory(&content, metadata).await?;
        }

        Ok(todo)
    }

    pub async fn update_todo_status(&self, todo_id: &str, status: TodoStatus) -> anyhow::Result<()> {
        // Search for the TODO in memory
        let results = self.storage.search(&format!("TODO id: {}", todo_id), Some(1), Some(0.9)).await?;

        if let Some(result) = results.first() {
            // Update metadata
            let mut metadata = result.point.metadata.clone();
            metadata.insert("status".to_string(), serde_json::Value::String(format!("{:?}", status)));
            metadata.insert("updated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

            // Create updated content
            let content = format!("TODO: {}", result.point.content.replace("TODO:", "UPDATED TODO:"));
            self.storage.add_memory(&content, metadata).await?;
        }

        Ok(())
    }

    pub async fn search_todos(&self, query: &str, status_filter: Option<TodoStatus>) -> anyhow::Result<Vec<TodoItem>> {
        let mut todos = Vec::new();

        // Search for TODO-related content
        let search_query = format!("TODO {}", query);
        let results = self.storage.search(&search_query, Some(20), Some(0.5)).await?;

        for result in results {
            if result.point.content.starts_with("TODO:") || result.point.content.starts_with("UPDATED TODO:") {
                let mut metadata = result.point.metadata.clone();
                let status = metadata.remove("status")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .and_then(|s| match s.as_str() {
                        "Pending" => Some(TodoStatus::Pending),
                        "InProgress" => Some(TodoStatus::InProgress),
                        "Completed" => Some(TodoStatus::Completed),
                        "Cancelled" => Some(TodoStatus::Cancelled),
                        _ => Some(TodoStatus::Pending),
                    });

                // Apply status filter if provided
                if let Some(ref filter_status) = status_filter {
                    if status.as_ref() != Some(filter_status) {
                        continue;
                    }
                }

                let todo = TodoItem {
                    id: result.point.id,
                    title: result.point.content
                        .replace("TODO:", "")
                        .replace("UPDATED TODO:", "")
                        .trim()
                        .to_string(),
                    description: metadata.remove("description")
                        .and_then(|v| v.as_str().map(|s| s.to_string())),
                    status: status.unwrap_or(TodoStatus::Pending),
                    priority: metadata.remove("priority")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .and_then(|s| match s.as_str() {
                            "Low" => Some(TodoPriority::Low),
                            "Medium" => Some(TodoPriority::Medium),
                            "High" => Some(TodoPriority::High),
                            "Critical" => Some(TodoPriority::Critical),
                            _ => Some(TodoPriority::Medium),
                        })
                        .unwrap_or(TodoPriority::Medium),
                    created_at: result.point.timestamp,
                    updated_at: metadata.remove("updated_at")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or(result.point.timestamp),
                    tags: metadata.remove("tags")
                        .and_then(|v| v.as_array().cloned())
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect())
                        .unwrap_or_default(),
                    metadata,
                };

                todos.push(todo);
            }
        }

        Ok(todos)
    }

    pub async fn get_todos_by_session_id(&self, session_id: &str, status_filter: Option<TodoStatus>) -> anyhow::Result<Vec<TodoItem>> {
        let mut todos = Vec::new();

        // Search for all TODO-related content
        let results = self.storage.search("TODO", Some(100), Some(0.3)).await?;

        for result in results {
            if result.point.content.starts_with("TODO:") || result.point.content.starts_with("UPDATED TODO:") {
                let mut metadata = result.point.metadata.clone();

                // Check if this TODO belongs to the requested session_id
                let todo_session_id = metadata.get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Filter by session_id
                if todo_session_id.as_ref() != Some(&session_id.to_string()) {
                    continue;
                }

                let status = metadata.remove("status")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .and_then(|s| match s.as_str() {
                        "Pending" => Some(TodoStatus::Pending),
                        "InProgress" => Some(TodoStatus::InProgress),
                        "Completed" => Some(TodoStatus::Completed),
                        "Cancelled" => Some(TodoStatus::Cancelled),
                        _ => Some(TodoStatus::Pending),
                    });

                // Apply status filter if provided
                if let Some(ref filter_status) = status_filter {
                    if status.as_ref() != Some(filter_status) {
                        continue;
                    }
                }

                let todo = TodoItem {
                    id: result.point.id,
                    title: result.point.content
                        .replace("TODO:", "")
                        .replace("UPDATED TODO:", "")
                        .trim()
                        .to_string(),
                    description: metadata.remove("description")
                        .and_then(|v| v.as_str().map(|s| s.to_string())),
                    status: status.unwrap_or(TodoStatus::Pending),
                    priority: metadata.remove("priority")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .and_then(|s| match s.as_str() {
                            "Low" => Some(TodoPriority::Low),
                            "Medium" => Some(TodoPriority::Medium),
                            "High" => Some(TodoPriority::High),
                            "Critical" => Some(TodoPriority::Critical),
                            _ => Some(TodoPriority::Medium),
                        })
                        .unwrap_or(TodoPriority::Medium),
                    created_at: result.point.timestamp,
                    updated_at: metadata.remove("updated_at")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or(result.point.timestamp),
                    tags: metadata.remove("tags")
                        .and_then(|v| v.as_array().cloned())
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect())
                        .unwrap_or_default(),
                    metadata,
                };

                todos.push(todo);
            }
        }

        Ok(todos)
    }

    fn detect_todo_type(&self, text: &str) -> String {
        if text.contains("fix") || text.contains("bug") {
            "bug".to_string()
        } else if text.contains("improve") || text.contains("enhance") {
            "improvement".to_string()
        } else if text.contains("refactor") || text.contains("clean") {
            "refactor".to_string()
        } else if text.contains("feature") || text.contains("add") {
            "feature".to_string()
        } else if text.contains("test") || text.contains("testing") {
            "testing".to_string()
        } else if text.contains("doc") || text.contains("documentation") {
            "documentation".to_string()
        } else {
            "general".to_string()
        }
    }
}