//! Embedding service using Ollama

use ollama_rs::{
    Ollama,
    generation::embeddings::request::GenerateEmbeddingsRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    pub embedding: Vec<f32>,
    pub model: String,
}

#[derive(Clone)]
pub struct EmbeddingService {
    client: Ollama,
    model: String,
}

impl EmbeddingService {
    pub fn new(ollama_url: &str, model: &str) -> Self {
        // 根据 ollama-rs 文档，需要传入完整的 host（包含协议）和端口
        let (host, port) = if ollama_url.starts_with("http://") || ollama_url.starts_with("https://") {
            let url = reqwest::Url::parse(ollama_url).unwrap_or_else(|_| {
                reqwest::Url::parse("http://localhost:11434").expect("Default URL should be valid")
            });
            (
                format!("{}://{}", url.scheme(), url.host_str().unwrap_or("localhost")),
                url.port().unwrap_or(11434)
            )
        } else {
            // 假设是 hostname:port 格式，添加默认协议
            if let Some(colon_pos) = ollama_url.find(':') {
                let host = ollama_url[..colon_pos].to_string();
                let port = ollama_url[colon_pos + 1..].parse::<u16>().unwrap_or(11434);
                (format!("http://{}", host), port)
            } else {
                (format!("http://{}", ollama_url), 11434)
            }
        };

        let client = Ollama::new(host, port);
        Self {
            client,
            model: model.to_string(),
        }
    }

    pub async fn generate_embedding(&self, text: &str) -> anyhow::Result<EmbeddingResult> {
        let request = GenerateEmbeddingsRequest::new(
            self.model.clone(),
            text.to_string().into()
        );

        let response = self.client.generate_embeddings(request).await
            .map_err(|e| anyhow::anyhow!("Failed to generate embedding: {}", e))?;

        if response.embeddings.is_empty() || response.embeddings[0].is_empty() {
            return Err(anyhow::anyhow!("Empty embedding returned"));
        }

        Ok(EmbeddingResult {
            embedding: response.embeddings[0].clone(),
            model: self.model.clone(),
        })
    }

    pub async fn generate_batch_embeddings(
        &self,
        texts: &[String]
    ) -> anyhow::Result<Vec<EmbeddingResult>> {
        let mut results = Vec::with_capacity(texts.len());

        for text in texts {
            let embedding = self.generate_embedding(text).await?;
            results.push(embedding);
        }

        Ok(results)
    }

    pub async fn test_connection(&self) -> anyhow::Result<bool> {
        let test_text = "test";
        match self.generate_embedding(test_text).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}