use anyhow::{anyhow, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use memvdb::normalize;
use parking_lot::Mutex;
use std::sync::Arc;

/// Backend interface for embedding generation (allows mocking in tests).
pub trait EmbeddingBackend: Send + Sync {
    fn dimension(&self) -> usize;
    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// Thread-safe wrapper around FastEmbed text embeddings.
pub struct FastEmbedder {
    backend: Arc<dyn EmbeddingBackend>,
}

impl FastEmbedder {
    pub fn new(model_name: &str) -> Result<Self> {
        let model = resolve_model(model_name)?;
        let info = TextEmbedding::get_model_info(&model)?;
        let encoder = TextEmbedding::try_new(InitOptions::new(model.clone()))?;
        Ok(Self {
            backend: Arc::new(FastEmbedBackend::new(encoder, info.dim)),
        })
    }

    /// Construct an embedder from a custom backend (used for deterministic tests).
    pub fn with_backend(backend: Arc<dyn EmbeddingBackend>) -> Self {
        Self { backend }
    }

    pub fn dimension(&self) -> usize {
        self.backend.dimension()
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()])?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No embedding generated"))
    }

    pub fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        self.backend.embed_batch(inputs)
    }
}

struct FastEmbedBackend {
    encoder: Arc<Mutex<TextEmbedding>>,
    dimension: usize,
}

impl FastEmbedBackend {
    fn new(encoder: TextEmbedding, dimension: usize) -> Self {
        Self {
            encoder: Arc::new(Mutex::new(encoder)),
            dimension,
        }
    }
}

impl EmbeddingBackend for FastEmbedBackend {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let mut encoder = self.encoder.lock();
        let embeddings = encoder.embed(inputs.to_vec(), None)?;
        Ok(embeddings
            .into_iter()
            .map(|vector| normalize(&vector))
            .collect())
    }
}

/// Simple embedding backend that returns deterministic vectors for tests.
pub struct MockEmbeddingBackend {
    dimension: usize,
    generator: Arc<dyn Fn(&str) -> Vec<f32> + Send + Sync>,
}

impl MockEmbeddingBackend {
    pub fn new<F>(dimension: usize, generator: F) -> Self
    where
        F: Fn(&str) -> Vec<f32> + Send + Sync + 'static,
    {
        Self {
            dimension,
            generator: Arc::new(generator),
        }
    }
}

impl EmbeddingBackend for MockEmbeddingBackend {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(inputs.len());
        for text in inputs {
            let vector = (self.generator)(text);
            if vector.len() != self.dimension {
                return Err(anyhow!(
                    "Mock embedding generated {}-dim vector, expected {}",
                    vector.len(),
                    self.dimension
                ));
            }
            results.push(normalize(&vector));
        }
        Ok(results)
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
