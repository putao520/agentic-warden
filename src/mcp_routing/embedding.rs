use anyhow::{anyhow, Result};

/// Backend interface for embedding generation (allows mocking in tests).
pub trait EmbeddingBackend: Send + Sync {
    fn dimension(&self) -> usize;
    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// Simple embedding backend that returns deterministic vectors for tests.
pub struct MockEmbeddingBackend {
    dimension: usize,
    generator: std::sync::Arc<dyn Fn(&str) -> Vec<f32> + Send + Sync>,
}

impl MockEmbeddingBackend {
    pub fn new<F>(dimension: usize, generator: F) -> Self
    where
        F: Fn(&str) -> Vec<f32> + Send + Sync + 'static,
    {
        Self {
            dimension,
            generator: std::sync::Arc::new(generator),
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
            results.push(memvdb::normalize(&vector));
        }
        Ok(results)
    }
}