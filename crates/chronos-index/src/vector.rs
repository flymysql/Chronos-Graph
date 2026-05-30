//! Vector index.
//!
//! A real brute-force cosine-similarity index (exact kNN). An HNSW backend with
//! segmented builds + background merge (design 6.2) is planned for scale; the
//! brute-force index is correct and is the reference the approximate index will
//! be validated against.

use crate::Filter;
use chronos_common::{Error, Result, VectorId};
use std::collections::HashMap;

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

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(a: &[f32]) -> f32 {
    dot(a, a).sqrt()
}

/// Cosine similarity in `[-1, 1]`; `0.0` if either vector is zero-length.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (na, nb) = (norm(a), norm(b));
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot(a, b) / (na * nb)
    }
}

/// Exact brute-force vector index.
#[derive(Default)]
pub struct BruteForceVectorIndex {
    dim: Option<usize>,
    vectors: HashMap<VectorId, Vec<f32>>,
}

impl BruteForceVectorIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }
}

impl VectorIndex for BruteForceVectorIndex {
    fn add(&mut self, id: VectorId, v: &[f32]) -> Result<()> {
        match self.dim {
            Some(d) if d != v.len() => {
                return Err(Error::Storage(format!(
                    "vector dim mismatch: index is {d}, got {}",
                    v.len()
                )));
            }
            None => self.dim = Some(v.len()),
            _ => {}
        }
        self.vectors.insert(id, v.to_vec());
        Ok(())
    }

    fn search(&self, q: &[f32], k: usize, _filter: &Filter) -> Result<Vec<VectorHit>> {
        let mut hits: Vec<VectorHit> = self
            .vectors
            .iter()
            .map(|(id, v)| VectorHit {
                id: *id,
                score: cosine(q, v),
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(k);
        Ok(hits)
    }

    fn rebuild_segment(&mut self) -> Result<()> {
        // Brute-force index has no segments to merge; a no-op that keeps the
        // contract the HNSW backend will implement.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_neighbor_ranks_by_cosine() {
        let mut idx = BruteForceVectorIndex::new();
        idx.add(VectorId::new(1), &[1.0, 0.0]).unwrap();
        idx.add(VectorId::new(2), &[0.0, 1.0]).unwrap();
        idx.add(VectorId::new(3), &[0.9, 0.1]).unwrap();

        let hits = idx.search(&[1.0, 0.0], 2, &Filter::default()).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, VectorId::new(1));
        assert_eq!(hits[1].id, VectorId::new(3));
    }

    #[test]
    fn dim_mismatch_errors() {
        let mut idx = BruteForceVectorIndex::new();
        idx.add(VectorId::new(1), &[1.0, 0.0]).unwrap();
        assert!(idx.add(VectorId::new(2), &[1.0, 0.0, 0.0]).is_err());
    }
}
