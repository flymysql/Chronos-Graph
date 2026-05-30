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
