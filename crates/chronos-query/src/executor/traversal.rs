//! Graph traversal operators: exact BFS and vector-guided semantic walk.
//!
//! Semantic traversal turns "should I expand this edge?" from a boolean pattern
//! match into a relevance-scored decision, enabling fuzzy multi-hop expansion.

use chronos_common::{AsOf, NodeId, Result};
use chronos_graph_model::Subgraph;

pub trait Traversal {
    /// Expand from `seeds` up to `depth`, keeping edges active at `at`, scoring
    /// candidate edges for semantic relevance to the query.
    fn expand(&self, seeds: &[NodeId], depth: u32, at: AsOf) -> Result<Subgraph>;
}
