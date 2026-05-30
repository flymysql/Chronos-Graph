//! [Phase 2] Community detection + incremental summary materialized views.
//!
//! Communities support "global" topic-level questions. New data triggers only
//! a *local* recomputation of affected communities, instead of a full rebuild
//! (the key cost advantage over batch GraphRAG).

use chronos_common::{NodeId, Result};
use chronos_graph_model::Community;

pub trait CommunityIndex: Send + Sync {
    /// Recompute communities affected by a set of changed nodes.
    fn incremental_update(&mut self, changed: &[NodeId]) -> Result<()>;
    /// Communities at a given hierarchy level.
    fn communities_at_level(&self, level: u8) -> Result<Vec<Community>>;
}
