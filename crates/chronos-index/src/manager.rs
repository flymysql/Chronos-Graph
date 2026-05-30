//! Embedding lifecycle management.
//!
//! A node/fact may carry multiple embeddings (different models / granularities).
//! When the embedding model changes, the manager re-embeds in the background
//! without taking the engine offline.

use chronos_common::Result;

/// Identifies an embedding model + version, so multiple can coexist per item.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EmbeddingModel {
    pub name: String,
    pub dim: usize,
}

pub trait IndexManager: Send + Sync {
    /// Register a new embedding model and kick off background re-embedding.
    fn register_model(&mut self, model: EmbeddingModel) -> Result<()>;
    /// Fraction (0.0..=1.0) of items re-embedded for the active model.
    fn reembed_progress(&self) -> f32;
}

/// A minimal in-memory manager: the active model is whatever was registered
/// last, and re-embedding is treated as instantaneous (synchronous build). The
/// background re-embedding pipeline is future work.
#[derive(Default)]
pub struct InMemoryIndexManager {
    models: Vec<EmbeddingModel>,
}

impl InMemoryIndexManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_model(&self) -> Option<&EmbeddingModel> {
        self.models.last()
    }
}

impl IndexManager for InMemoryIndexManager {
    fn register_model(&mut self, model: EmbeddingModel) -> Result<()> {
        if !self.models.contains(&model) {
            self.models.push(model);
        }
        Ok(())
    }

    fn reembed_progress(&self) -> f32 {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_active_model() {
        let mut m = InMemoryIndexManager::new();
        assert!(m.active_model().is_none());
        m.register_model(EmbeddingModel {
            name: "e5".into(),
            dim: 384,
        })
        .unwrap();
        assert_eq!(m.active_model().unwrap().dim, 384);
        assert_eq!(m.reembed_progress(), 1.0);
    }
}
