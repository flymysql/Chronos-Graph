//! The three hierarchical tiers and the `Subgraph` returned by retrieval.

use crate::{Edge, Node};
use chronos_common::{ChunkId, DocId, NodeId, Timestamp};

/// Which tier a piece of the graph belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Raw ingested unit (message / document chunk).
    Episode,
    /// Resolved semantic entities and the facts between them.
    Entity,
    /// Clusters of entities with rolled-up summaries.
    Community,
}

/// An ingested unit of source data. `reference_time` anchors extracted facts'
/// `valid_from` (so "last Thursday" maps to real-world time, not ingest time).
#[derive(Debug, Clone)]
pub struct Episode {
    pub doc: DocId,
    pub chunk: ChunkId,
    pub text: String,
    pub reference_time: Timestamp,
}

/// A detected community with an incrementally maintained summary (Phase 2).
#[derive(Debug, Clone)]
pub struct Community {
    pub id: u64,
    pub members: Vec<NodeId>,
    pub summary: Option<String>,
    /// Resolution level in the hierarchical community structure.
    pub level: u8,
}

/// A retrieved, possibly multi-tier slice of the graph, ready to be ranked,
/// budget-trimmed and serialized into LLM context.
#[derive(Debug, Default, Clone)]
pub struct Subgraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub episodes: Vec<Episode>,
    pub communities: Vec<Community>,
}

impl Subgraph {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
            && self.edges.is_empty()
            && self.episodes.is_empty()
            && self.communities.is_empty()
    }
}
