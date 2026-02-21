use crate::mcp_routing::models::{MethodVectorRecord, ToolVectorRecord};
use anyhow::{anyhow, Result};
use memvdb::{CacheDB, Distance, Embedding, SimilarityResult};
use std::collections::HashMap;

const TOOLS_COLLECTION: &str = "mcp_tools";
const METHODS_COLLECTION: &str = "mcp_methods";

pub struct ToolEmbedding {
    pub record: ToolVectorRecord,
    pub vector: Vec<f32>,
}

pub struct MethodEmbedding {
    pub record: MethodVectorRecord,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct ScoredTool {
    pub server: String,
    pub tool: String,
    pub description: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct ScoredMethod {
    pub server: String,
    pub tool: String,
    pub metadata: HashMap<String, String>,
}

pub struct MemRoutingIndex {
    db: CacheDB,
    dimension: usize,
}

impl MemRoutingIndex {
    pub fn new(dimension: usize) -> Result<Self> {
        let mut db = CacheDB::new();
        db.create_collection(TOOLS_COLLECTION.to_string(), dimension, Distance::Cosine)?;
        db.create_collection(METHODS_COLLECTION.to_string(), dimension, Distance::Cosine)?;
        Ok(Self { db, dimension })
    }

    pub fn rebuild(&mut self, tools: &[ToolEmbedding], methods: &[MethodEmbedding]) -> Result<()> {
        self.db = CacheDB::new();
        self.db.create_collection(
            TOOLS_COLLECTION.to_string(),
            self.dimension,
            Distance::Cosine,
        )?;
        self.db.create_collection(
            METHODS_COLLECTION.to_string(),
            self.dimension,
            Distance::Cosine,
        )?;

        for tool in tools {
            self.db
                .insert_into_collection(TOOLS_COLLECTION, embedding_from_tool(tool)?)?;
        }
        for method in methods {
            self.db
                .insert_into_collection(METHODS_COLLECTION, embedding_from_method(method)?)?;
        }
        Ok(())
    }

    pub fn search_tools(&self, vector: &[f32], limit: usize) -> Result<Vec<ScoredTool>> {
        if vector.len() != self.dimension {
            return Err(anyhow!(
                "Search vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            ));
        }
        let tools = self
            .db
            .get_collection(TOOLS_COLLECTION)
            .ok_or_else(|| anyhow!("Tool collection not initialised"))?;
        let results = tools.get_similarity(&adapt_query(vector), limit);
        for r in &results {
            let tool_name = r.embedding.metadata.as_ref()
                .and_then(|m| m.get("tool"))
                .map(|s| s.as_str())
                .unwrap_or("?");
            eprintln!("   📊 score={:.4} tool={}", r.score, tool_name);
        }
        Ok(results
            .into_iter()
            .filter_map(scored_tool_from_result)
            .collect())
    }

    pub fn search_methods(&self, vector: &[f32], limit: usize) -> Result<Vec<ScoredMethod>> {
        if vector.len() != self.dimension {
            return Err(anyhow!(
                "Search vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            ));
        }
        let methods = self
            .db
            .get_collection(METHODS_COLLECTION)
            .ok_or_else(|| anyhow!("Method collection not initialised"))?;
        Ok(methods
            .get_similarity(&adapt_query(vector), limit)
            .into_iter()
            .filter_map(scored_method_from_result)
            .collect())
    }
}

fn embedding_from_tool(entry: &ToolEmbedding) -> Result<Embedding> {
    Ok(Embedding {
        id: HashMap::from([
            ("id".to_string(), entry.record.id.clone()),
            ("server".to_string(), entry.record.server.clone()),
            ("tool".to_string(), entry.record.tool_name.clone()),
        ]),
        vector: entry.vector.clone(),
        metadata: Some(entry.record.metadata.clone()),
    })
}

fn embedding_from_method(entry: &MethodEmbedding) -> Result<Embedding> {
    Ok(Embedding {
        id: HashMap::from([
            ("id".to_string(), entry.record.id.clone()),
            ("server".to_string(), entry.record.server.clone()),
            ("tool".to_string(), entry.record.tool_name.clone()),
        ]),
        vector: entry.vector.clone(),
        metadata: Some(entry.record.metadata.clone()),
    })
}

fn scored_tool_from_result(result: SimilarityResult) -> Option<ScoredTool> {
    let score = result.score;
    let embedding = result.embedding;
    let metadata = embedding.metadata?;
    let server = metadata.get("server")?.clone();
    let tool = metadata.get("tool")?.clone();
    let description = metadata.get("description").cloned();
    Some(ScoredTool {
        server,
        tool,
        description,
        score,
    })
}

fn scored_method_from_result(result: SimilarityResult) -> Option<ScoredMethod> {
    let embedding = result.embedding;
    let metadata = embedding.metadata?;
    let server = metadata.get("server")?.clone();
    let tool = metadata.get("tool")?.clone();
    Some(ScoredMethod {
        server,
        tool,
        metadata,
    })
}

fn adapt_query(vector: &[f32]) -> Vec<f32> {
    vector.to_vec()
}
