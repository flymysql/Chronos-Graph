//! Engine configuration shared across crates.

use std::path::PathBuf;

/// Top-level engine configuration.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// On-disk data directory for the storage engine.
    pub data_dir: PathBuf,
    /// Dimensionality of stored embedding vectors.
    pub vector_dim: usize,
    /// Default hybrid-retrieval scoring weights.
    pub scoring: ScoringWeights,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./chronos-data"),
            vector_dim: 1536,
            scoring: ScoringWeights::default(),
        }
    }
}

/// Weights for the unified hybrid-retrieval cost model.
#[derive(Debug, Clone, Copy)]
pub struct ScoringWeights {
    pub vector: f32,
    pub bm25: f32,
    pub structural: f32,
    pub recency: f32,
    pub validity: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            vector: 1.0,
            bm25: 0.5,
            structural: 0.8,
            recency: 0.3,
            validity: 1.0,
        }
    }
}
