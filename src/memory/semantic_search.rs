//! Semantic search functionality

use super::{
    vector_store::{MemoryPoint, SearchResult, VectorStore},
    embedding::EmbeddingService
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct SemanticSearch {
    vector_store: VectorStore,
    embedding_service: EmbeddingService,
}

impl SemanticSearch {
    pub fn new(vector_store: VectorStore, embedding_service: EmbeddingService) -> Self {
        Self {
            vector_store,
            embedding_service,
        }
    }

    pub async fn search(
        &self,
        query: &str,
        limit: Option<u64>,
        score_threshold: Option<f32>,
    ) -> anyhow::Result<Vec<SearchResult>> {
        // Generate embedding for query
        let embedding_result = self.embedding_service.generate_embedding(query).await?;

        // Search vector store
        self.vector_store.search(
            embedding_result.embedding,
            limit,
            score_threshold,
        ).await
    }

    pub async fn add_memory(
        &self,
        content: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> anyhow::Result<String> {
        // Generate embedding
        let embedding_result = self.embedding_service.generate_embedding(content).await?;

        // Create memory point
        let point = MemoryPoint {
            id: String::new(), // Will be set by vector store
            content: content.to_string(),
            metadata,
            timestamp: chrono::Utc::now(),
        };

        // Store in vector store
        self.vector_store.upsert_point(point, embedding_result.embedding).await
    }

    pub async fn search_related_conversations(
        &self,
        current_conversation: &str,
        limit: Option<u64>,
    ) -> anyhow::Result<Vec<SearchResult>> {
        // Split conversation into chunks for better search
        let chunks = self.split_into_chunks(current_conversation, 200);
        let chunks_len = chunks.len();
        let mut all_results = Vec::new();

        for chunk in chunks {
            let chunk_limit = limit.map(|l| l / chunks_len as u64);
            let results = self.search(&chunk, chunk_limit, Some(0.5)).await?;
            all_results.extend(results);
        }

        // Remove duplicates and sort by score
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        all_results.dedup_by(|a, b| a.point.id == b.point.id);
        all_results.truncate(limit.unwrap_or(10) as usize);

        Ok(all_results)
    }

    fn split_into_chunks(&self, text: &str, max_chunk_size: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let sentences: Vec<&str> = text.split(&['.', '?', '!', '\n'][..]).collect();
        let mut current_chunk = String::new();

        for sentence in sentences {
            let trimmed = sentence.trim();
            if trimmed.is_empty() {
                continue;
            }

            if current_chunk.len() + trimmed.len() > max_chunk_size && !current_chunk.is_empty() {
                chunks.push(current_chunk.clone());
                current_chunk.clear();
            }

            current_chunk.push_str(trimmed);
            current_chunk.push('.');
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }
}