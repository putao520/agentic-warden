use anyhow::{anyhow, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use memvdb::normalize;
use parking_lot::Mutex;
use std::sync::Arc;

/// Thread-safe wrapper around FastEmbed text embeddings.
pub struct FastEmbedder {
    encoder: Arc<Mutex<TextEmbedding>>,
    dimension: usize,
}

impl FastEmbedder {
    pub fn new(model_name: &str) -> Result<Self> {
        let model = resolve_model(model_name)?;
        let dimension = TextEmbedding::get_model_info(&model)?.dim;
        let encoder = TextEmbedding::try_new(InitOptions::new(model.clone()))?;
        Ok(Self {
            encoder: Arc::new(Mutex::new(encoder)),
            dimension,
        })
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()])?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No embedding generated"))
    }

    pub fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let encoder = self.encoder.lock();
        let embeddings = encoder.embed(inputs.to_vec(), None)?;
        Ok(embeddings
            .into_iter()
            .map(|vector| normalize(&vector))
            .collect())
    }
}

fn resolve_model(name: &str) -> Result<EmbeddingModel> {
    let lowered = name.trim().to_lowercase();
    for info in TextEmbedding::list_supported_models() {
        if info.model_code.to_lowercase() == lowered
            || format!("{:?}", info.model).to_lowercase() == lowered
        {
            return Ok(info.model);
        }
        // Accept shorthand without vendor prefix.
        if let Some(short) = info
            .model_code
            .split('/')
            .last()
            .map(|segment| segment.to_lowercase())
        {
            if short == lowered {
                return Ok(info.model);
            }
        }
    }
    Err(anyhow!(
        "FastEmbed model '{}' is not supported by fastembed-rs",
        name
    ))
}
