//! Secondary indexes used by hybrid retrieval.

pub mod fulltext;
pub mod manager;
pub mod vector;

pub use fulltext::{tokenize, Bm25Hit, Bm25Index, FullTextIndex};
pub use manager::{EmbeddingModel, InMemoryIndexManager, IndexManager};
pub use vector::{cosine, BruteForceVectorIndex, VectorHit, VectorIndex};

/// A filter applied during index search (e.g. tenant / validity predicates
/// pushed down into the scan).
#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub tenant: Option<u64>,
    /// When set, only return items whose facts are active at this instant.
    pub active_at: Option<chronos_common::AsOf>,
}
