use super::{official::OfficialRegistrySource, smithery::SmitherySource, source::RegistrySource};
use crate::commands::mcp::McpServerConfig;
use anyhow::{anyhow, Result};
use futures::future::join_all;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

use super::types::{McpServerDetail, McpServerInfo};

const CACHE_TTL: Duration = Duration::from_secs(3600);

pub struct RegistryAggregator {
    sources: Vec<Box<dyn RegistrySource>>,
    cache: Arc<RwLock<HashMap<CacheKey, CachedEntry>>>,
}

impl RegistryAggregator {
    pub fn new() -> Self {
        let mut sources: Vec<Box<dyn RegistrySource>> = Vec::new();
        sources.push(Box::new(OfficialRegistrySource::new()));
        sources.push(Box::new(SmitherySource::new()));
        Self::with_sources(sources)
    }

    pub fn with_sources(sources: Vec<Box<dyn RegistrySource>>) -> Self {
        Self {
            sources,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        source_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<McpServerInfo>> {
        let filter = source_filter.map(|s| s.to_lowercase());
        let key = CacheKey::new(query, filter.clone(), limit);

        if let Some(cached) = self.cache.read().await.get(&key) {
            if cached.created_at.elapsed() < CACHE_TTL {
                return Ok(cached.results.clone());
            }
        }

        let sources = self.filtered_sources(filter.as_deref());
        if sources.is_empty() {
            return Err(anyhow!("No registry source matched the filter"));
        }

        let mut tasks = Vec::new();
        for source in &sources {
            tasks.push(source.search(query, limit));
        }

        let results = join_all(tasks).await;
        let mut merged_inputs = Vec::new();
        let mut errors = Vec::new();
        for (idx, result) in results.into_iter().enumerate() {
            match result {
                Ok(list) => merged_inputs.push((sources[idx].priority(), list)),
                Err(err) => errors.push((sources[idx].source_id().to_string(), err)),
            }
        }

        if merged_inputs.is_empty() {
            if let Some((source, err)) = errors.into_iter().next() {
                return Err(anyhow!("{} search failed: {}", source, err));
            }
            return Err(anyhow!("No results returned from registry sources"));
        }

        let merged = merge_results(merged_inputs);
        self.cache.write().await.insert(
            key,
            CachedEntry {
                created_at: Instant::now(),
                results: merged.clone(),
            },
        );

        Ok(merged)
    }

    pub async fn get_server_detail(
        &self,
        qualified_name: &str,
        source_filter: Option<&str>,
    ) -> Result<McpServerDetail> {
        let filter = normalize_source_filter(qualified_name, source_filter);
        let sources = self.filtered_sources(filter.as_deref());

        if sources.is_empty() {
            return Err(anyhow!("No registry source available for detail lookup"));
        }

        let mut last_error: Option<anyhow::Error> = None;
        for source in sources {
            match source.get_server(qualified_name).await {
                Ok(Some(detail)) => return Ok(detail),
                Ok(None) => continue,
                Err(err) => last_error = Some(err),
            }
        }

        if let Some(err) = last_error {
            Err(anyhow!("Failed to fetch server detail: {}", err))
        } else {
            Err(anyhow!(
                "Server '{}' not found in selected registry sources",
                qualified_name
            ))
        }
    }

    pub async fn get_install_config(
        &self,
        qualified_name: &str,
        source_filter: Option<&str>,
    ) -> Result<McpServerConfig> {
        let filter = normalize_source_filter(qualified_name, source_filter);
        let sources = self.filtered_sources(filter.as_deref());

        if sources.is_empty() {
            return Err(anyhow!("No registry source available for install"));
        }

        let mut last_error: Option<anyhow::Error> = None;
        for source in sources {
            match source.get_install_config(qualified_name).await {
                Ok(cfg) => return Ok(cfg),
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow!(
                "Server '{}' not found in any selected registry source",
                qualified_name
            )
        }))
    }

    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    fn filtered_sources(&self, filter: Option<&str>) -> Vec<&Box<dyn RegistrySource>> {
        match filter {
            Some(target) => self
                .sources
                .iter()
                .filter(|src| src.source_id().eq_ignore_ascii_case(target))
                .collect(),
            None => self.sources.iter().collect(),
        }
    }
}

fn merge_results(inputs: Vec<(u8, Vec<McpServerInfo>)>) -> Vec<McpServerInfo> {
    let mut map: HashMap<String, (u8, McpServerInfo)> = HashMap::new();

    for (priority, list) in inputs {
        for info in list {
            match map.get(&info.qualified_name) {
                Some((existing_priority, _)) if *existing_priority <= priority => continue,
                _ => {
                    map.insert(info.qualified_name.clone(), (priority, info));
                }
            }
        }
    }

    let mut merged: Vec<(u8, McpServerInfo)> = map.into_values().collect();
    merged.sort_by(|(pa, ia), (pb, ib)| {
        pa.cmp(pb)
            .then_with(|| ib.downloads.cmp(&ia.downloads))
            .then_with(|| ia.display_name.cmp(&ib.display_name))
    });
    merged.into_iter().map(|(_, info)| info).collect()
}

#[derive(Clone)]
struct CacheKey {
    query: String,
    source: Option<String>,
    limit: usize,
}

impl CacheKey {
    fn new(query: &str, source: Option<String>, limit: usize) -> Self {
        Self {
            query: query.to_string(),
            source,
            limit,
        }
    }
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query && self.source == other.source && self.limit == other.limit
    }
}

impl Eq for CacheKey {}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.query.hash(state);
        self.source.hash(state);
        self.limit.hash(state);
    }
}

struct CachedEntry {
    created_at: Instant,
    results: Vec<McpServerInfo>,
}

fn normalize_source_filter(name: &str, filter: Option<&str>) -> Option<String> {
    if let Some((prefix, _)) = name.split_once(':') {
        return Some(prefix.to_lowercase());
    }
    filter.map(|s| s.to_lowercase())
}
