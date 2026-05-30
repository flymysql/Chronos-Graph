//! Unified hybrid retrieval: vector + BM25 + semantic traversal, ranked by one
//! engine-level cost model and returned as a budget-bounded subgraph in a
//! single call (enables server-side multi-hop).

use chronos_common::{AsOf, Result, TokenBudget};
use chronos_graph_model::Subgraph;

/// Weights for the unified relevance score:
/// `score = w_vec*sim + w_bm25*bm25 + w_struct*proximity + w_recency*r + w_validity*v(t)`.
#[derive(Debug, Clone, Copy)]
pub struct HybridScorer {
    pub w_vec: f32,
    pub w_bm25: f32,
    pub w_struct: f32,
    pub w_recency: f32,
    pub w_validity: f32,
}

impl Default for HybridScorer {
    fn default() -> Self {
        Self {
            w_vec: 1.0,
            w_bm25: 0.5,
            w_struct: 0.8,
            w_recency: 0.3,
            w_validity: 1.0,
        }
    }
}

impl HybridScorer {
    /// Combine per-signal scores into a single relevance value.
    pub fn score(&self, vec: f32, bm25: f32, structural: f32, recency: f32, validity: f32) -> f32 {
        self.w_vec * vec
            + self.w_bm25 * bm25
            + self.w_struct * structural
            + self.w_recency * recency
            + self.w_validity * validity
    }
}

/// The retrieval entry point used by the server and MCP layers.
pub trait RetrievalOperator {
    fn retrieve(
        &self,
        query: &crate::CompiledQuery,
        budget: TokenBudget,
        at: AsOf,
    ) -> Result<Subgraph>;
}
