use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use sahomedb::{
    collection::{Collection, Config, Record, SearchResult},
    database::Database,
    metadata::Metadata,
    vector::Vector,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

const COLLECTION_NAME: &str = "conversation_history";

/// Canonical structure for recorded Claude Code conversations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tools_used: Vec<String>,
}

impl ConversationRecord {
    pub fn new(
        session_id: Option<String>,
        role: impl Into<String>,
        content: impl Into<String>,
        tools_used: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
            tools_used,
        }
    }
}

/// Search result containing a conversation record and its similarity score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchResult {
    pub record: ConversationRecord,
    /// Similarity score (0.0 to 1.0, higher is more similar).
    /// Calculated as (1.0 - distance) for cosine distance.
    pub similarity_score: f32,
}

struct HistoryState {
    db: Database,
    collection: Collection,
    dimension: usize,
}

/// File-backed storage built on top of SahomeDB.
pub struct ConversationHistoryStore {
    state: Mutex<HistoryState>,
}

impl ConversationHistoryStore {
    pub fn new(db_path: &PathBuf, dimension: usize) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut db = Database::open(db_path.to_string_lossy().as_ref())
            .map_err(|e| anyhow!(e.message().to_string()))?;
        let collection = match db.get_collection(COLLECTION_NAME) {
            Ok(mut collection) => {
                if collection.dimension() == 0 {
                    collection
                        .set_dimension(dimension)
                        .map_err(|e| anyhow!(e.message().to_string()))?;
                } else if collection.dimension() != dimension {
                    return Err(anyhow!(
                        "Conversation history dimension mismatch: expected {}, found {}",
                        dimension,
                        collection.dimension()
                    ));
                }
                collection
            }
            Err(_) => {
                let mut config = Config::default();
                config
                    .set_distance("cosine")
                    .map_err(|e| anyhow!(e.message().to_string()))?;
                let mut collection = Collection::new(&config);
                collection
                    .set_dimension(dimension)
                    .map_err(|e| anyhow!(e.message().to_string()))?;
                collection.set_relevancy(0.0);
                db.save_collection(COLLECTION_NAME, &collection)
                    .map_err(|e| anyhow!(e.message().to_string()))?;

                // Re-open to ensure persistence
                db.get_collection(COLLECTION_NAME)
                    .map_err(|e| anyhow!(e.message().to_string()))?
            }
        };

        Ok(Self {
            state: Mutex::new(HistoryState {
                db,
                collection,
                dimension,
            }),
        })
    }

    pub fn append(&self, record: ConversationRecord, embedding: Vec<f32>) -> Result<()> {
        let metadata = Metadata::from(record);
        let state = &mut *self.state.lock();
        if embedding.len() != state.dimension {
            return Err(anyhow!(
                "Embedding dimension mismatch: expected {}, got {}",
                state.dimension,
                embedding.len()
            ));
        }

        let vector = Vector::from(embedding);
        let entry = Record::new(&vector, &metadata);
        state
            .collection
            .insert(&entry)
            .map_err(|e| anyhow!(e.message().to_string()))?;
        let snapshot = state.collection.clone();
        state
            .db
            .save_collection(COLLECTION_NAME, &snapshot)
            .map_err(|e| anyhow!(e.message().to_string()))?;
        Ok(())
    }

    pub fn top_conversations(
        &self,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<ConversationRecord>> {
        let state = self.state.lock();
        if embedding.len() != state.dimension {
            return Err(anyhow!(
                "Embedding dimension mismatch: expected {}, got {}",
                state.dimension,
                embedding.len()
            ));
        }
        let results = state
            .collection
            .search(&Vector::from(embedding), limit)
            .map_err(|e| anyhow!(e.message().to_string()))?;
        let records: Vec<ConversationRecord> = results
            .into_iter()
            .filter_map(record_from_search)
            .collect();
        Ok(records)
    }

    /// Search for conversations with similarity scores.
    pub fn search_with_scores(
        &self,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<ConversationSearchResult>> {
        let state = self.state.lock();
        if embedding.len() != state.dimension {
            return Err(anyhow!(
                "Embedding dimension mismatch: expected {}, got {}",
                state.dimension,
                embedding.len()
            ));
        }
        let results = state
            .collection
            .search(&Vector::from(embedding), limit)
            .map_err(|e| anyhow!(e.message().to_string()))?;

        let search_results: Vec<ConversationSearchResult> = results
            .into_iter()
            .filter_map(|result| {
                let distance = result.distance;
                let record = record_from_search(result)?;
                // Convert distance to similarity (cosine distance: 0 = identical, 2 = opposite)
                // Similarity: 1.0 - (distance / 2.0) gives range [0.0, 1.0]
                let similarity_score = 1.0 - (distance / 2.0).min(1.0).max(0.0);
                Some(ConversationSearchResult {
                    record,
                    similarity_score,
                })
            })
            .collect();
        Ok(search_results)
    }
}

fn record_from_search(result: SearchResult) -> Option<ConversationRecord> {
    match result.data {
        Metadata::Object(map) => Some(ConversationRecord {
            id: as_string(&map, "id")?,
            session_id: as_string(&map, "session_id"),
            role: as_string(&map, "role")?,
            content: as_string(&map, "content")?,
            timestamp: as_string(&map, "timestamp")
                .and_then(|ts| ts.parse::<DateTime<Utc>>().ok())
                .unwrap_or_else(Utc::now),
            tools_used: as_array(&map, "tools_used"),
        }),
        _ => None,
    }
}

fn as_string(map: &HashMap<String, Metadata>, key: &str) -> Option<String> {
    match map.get(key) {
        Some(Metadata::Text(value)) => Some(value.clone()),
        Some(Metadata::Integer(value)) => Some(value.to_string()),
        Some(Metadata::Float(value)) => Some(value.to_string()),
        _ => None,
    }
}

fn as_array(map: &HashMap<String, Metadata>, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(Metadata::Array(values)) => values
            .iter()
            .filter_map(|item| match item {
                Metadata::Text(value) => Some(value.clone()),
                Metadata::Integer(value) => Some(value.to_string()),
                Metadata::Float(value) => Some(value.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

impl From<ConversationRecord> for Metadata {
    fn from(value: ConversationRecord) -> Self {
        let mut map: HashMap<String, Metadata> = HashMap::new();
        map.insert("id".into(), Metadata::from(value.id));
        if let Some(session) = value.session_id {
            map.insert("session_id".into(), Metadata::from(session));
        }
        map.insert("role".into(), Metadata::from(value.role));
        map.insert("content".into(), Metadata::from(value.content));
        map.insert(
            "timestamp".into(),
            Metadata::from(value.timestamp.to_rfc3339()),
        );
        map.insert(
            "tools_used".into(),
            Metadata::from(value.tools_used.clone()),
        );
        Metadata::Object(map)
    }
}
