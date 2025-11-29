use anyhow::{anyhow, Result};
use gllm::Client;
use memvdb::normalize;
use parking_lot::Mutex;
use std::sync::Arc;

/// Backend interface for embedding generation (allows mocking in tests).
pub trait EmbeddingBackend: Send + Sync {
    fn dimension(&self) -> usize;
    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// Thread-safe wrapper around gllm text embeddings.
pub struct FastEmbedder {
    backend: Arc<dyn EmbeddingBackend>,
}

impl FastEmbedder {
    /// Create a new embedder with the specified model name.
    pub fn new(model_name: &str) -> Result<Self> {
        let client = Client::new(model_name)
            .map_err(|e| anyhow!("Failed to create gllm embedding client: {}", e))?;
        Ok(Self {
            backend: Arc::new(GllmBackend::new(client)),
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

struct GllmBackend {
    client: Arc<Mutex<Client>>,
    dimension: usize,
}

impl GllmBackend {
    fn new(client: Client) -> Self {
        // Get dimension from client configuration
        // gllm models have fixed dimensions, we'll detect it from the model
        let dimension = 384; // Default for bge-small-en
        Self {
            client: Arc::new(Mutex::new(client)),
            dimension,
        }
    }
}

impl EmbeddingBackend for GllmBackend {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let client = self.client.lock();
        let response = client
            .embeddings(inputs)
            .generate()
            .map_err(|e| anyhow!("Embedding generation failed: {}", e))?;

        Ok(response
            .embeddings
            .into_iter()
            .map(|emb| normalize(&emb.embedding))
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

