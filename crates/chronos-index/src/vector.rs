//! Vector index abstraction (HNSW backend planned).

use crate::Filter;
use chronos_common::{Result, VectorId};

#[derive(Debug, Clone, Copy)]
pub struct VectorHit {
    pub id: VectorId,
    pub score: f32,
}

/// Approximate nearest-neighbor index over embeddings. Supports segmented
/// builds + background merge to absorb high write rates (see design 6.2).
pub trait VectorIndex: Send + Sync {
    fn add(&mut self, id: VectorId, v: &[f32]) -> Result<()>;
    fn search(&self, q: &[f32], k: usize, filter: &Filter) -> Result<Vec<VectorHit>>;
    /// Merge pending insert segments into the main graph.
    fn rebuild_segment(&mut self) -> Result<()>;
}
