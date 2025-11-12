//! Vector store using Qdrant HTTP REST API

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoint {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub point: MemoryPoint,
    pub score: f32,
}

// Qdrant HTTP REST API 结构体定义
#[derive(Debug, Serialize, Deserialize)]
struct CollectionsResponse {
    result: CollectionsResult,
    status: String,
    time: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CollectionsResult {
    collections: Vec<CollectionInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CollectionInfo {
    name: String,
}

#[derive(Debug, Serialize)]
struct CreateCollectionRequest {
    vectors: VectorConfig,
}

#[derive(Debug, Serialize)]
struct VectorConfig {
    size: usize,
    distance: String,
}

#[derive(Debug, Serialize)]
struct UpsertRequest {
    points: Vec<PointRequest>,
}

#[derive(Debug, Serialize)]
struct PointRequest {
    id: String,
    vector: Vec<f32>,
    payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct SearchRequest {
    vector: Vec<f32>,
    limit: u64,
    score_threshold: Option<f32>,
    with_payload: bool,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    result: Option<Vec<ScoredPoint>>,
    status: Option<SearchErrorStatus>,
}

#[derive(Debug, Deserialize)]
struct SearchErrorStatus {
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchSuccessStatus {
    #[serde(rename = "ok")]
    ok: bool,
}

#[derive(Debug, Deserialize)]
struct ScoredPoint {
    id: serde_json::Value,
    score: f32,
    payload: serde_json::Value,
}

#[derive(Clone)]
pub struct VectorStore {
    client: Client,
    base_url: String,
    collection_name: String,
}

impl VectorStore {
    pub fn new(qdrant_url: &str) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: qdrant_url.trim_end_matches('/').to_string(),
            collection_name: "agentic_warden_memory".to_string(),
        })
    }

    pub async fn initialize_collection(&self) -> anyhow::Result<()> {
        let url = format!("{}/collections", self.base_url);

        let response = self.client.get(&url).send().await
            .map_err(|e| anyhow::anyhow!("Failed to list collections: {}", e))?;

        if response.status() == 200 {
            let collections: CollectionsResponse = response.json().await
                .map_err(|e| anyhow::anyhow!("Failed to parse collections response: {}", e))?;

            let collection_exists = collections.result.collections
                .iter()
                .any(|c| c.name == self.collection_name);

            if !collection_exists {
                let create_url = format!("{}/collections/{}", self.base_url, self.collection_name);
                let create_request = CreateCollectionRequest {
                    vectors: VectorConfig {
                        size: 1536,
                        distance: "Cosine".to_string(),
                    },
                };

                let create_response = self.client.put(&create_url)
                    .json(&create_request)
                    .send().await
                    .map_err(|e| anyhow::anyhow!("Failed to create collection: {}", e))?;

                if create_response.status().is_success() {
                    println!("Created collection: {}", self.collection_name);
                } else {
                    return Err(anyhow::anyhow!("Failed to create collection: {}", create_response.status()));
                }
            }
        } else {
            return Err(anyhow::anyhow!("Failed to list collections: {}", response.status()));
        }

        Ok(())
    }

    pub async fn upsert_point(&self, point: MemoryPoint, embedding: Vec<f32>) -> anyhow::Result<String> {
        let point_id = Uuid::new_v4().to_string();

        let mut metadata = point.metadata.clone();
        metadata.insert("content".to_string(), serde_json::Value::String(point.content.clone()));
        metadata.insert("timestamp".to_string(), serde_json::Value::String(point.timestamp.to_rfc3339()));

        let point_request = PointRequest {
            id: point_id.clone(),
            vector: embedding,
            payload: serde_json::to_value(metadata)?,
        };

        let url = format!("{}/collections/{}/points", self.base_url, self.collection_name);
        let upsert_request = UpsertRequest {
            points: vec![point_request],
        };

        let response = self.client.put(&url)
            .json(&upsert_request)
            .send().await
            .map_err(|e| anyhow::anyhow!("Failed to upsert point: {}", e))?;

        if response.status().is_success() {
            Ok(point_id)
        } else {
            Err(anyhow::anyhow!("Failed to upsert point: {}", response.status()))
        }
    }

    pub async fn search(
        &self,
        query_embedding: Vec<f32>,
        limit: Option<u64>,
        score_threshold: Option<f32>,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let url = format!("{}/collections/{}/points/search", self.base_url, self.collection_name);

        let search_request = SearchRequest {
            vector: query_embedding,
            limit: limit.unwrap_or(10),
            score_threshold,
            with_payload: true,
        };

        let response = self.client.post(&url)
            .json(&search_request)
            .send().await
            .map_err(|e| anyhow::anyhow!("Failed to search points: {}", e))?;

        if response.status() != 200 {
            return Err(anyhow::anyhow!("Search failed with status: {}", response.status()));
        }

        let search_response: SearchResponse = response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse search response: {}", e))?;

        // Check for error status
        if let Some(status) = &search_response.status {
            if let Some(error) = &status.error {
                return Err(anyhow::anyhow!("Search API error: {}", error));
            }
        }

        let mut results = Vec::new();
        if let Some(result_points) = search_response.result {
            for scored_point in result_points {
                let payload = scored_point.payload.as_object()
                    .ok_or_else(|| anyhow::anyhow!("Invalid payload format"))?;

                let content = payload.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let timestamp_str = payload.get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or(chrono::Utc::now());

                let mut metadata = HashMap::new();
                for (k, v) in payload {
                    if k != "content" && k != "timestamp" {
                        metadata.insert(k.clone(), v.clone());
                    }
                }

                let memory_point = MemoryPoint {
                    id: scored_point.id.to_string(),
                    content,
                    metadata,
                    timestamp,
                };

                results.push(SearchResult {
                    point: memory_point,
                    score: scored_point.score,
                });
            }
        }

        Ok(results)
    }

    pub async fn delete_point(&self, point_id: &str) -> anyhow::Result<()> {
        let url = format!("{}/collections/{}/points", self.base_url, self.collection_name);

        let delete_request = serde_json::json!({
            "points": [point_id]
        });

        let response = self.client.delete(&url)
            .json(&delete_request)
            .send().await
            .map_err(|e| anyhow::anyhow!("Failed to delete point: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to delete point: {}", response.status()))
        }
    }

    pub async fn test_connection(&self) -> anyhow::Result<bool> {
        let url = format!("{}/collections", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status() == 200),
            Err(_) => Ok(false),
        }
    }
}