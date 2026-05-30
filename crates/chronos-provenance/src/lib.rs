//! Provenance: a first-class structure linking facts back to their source text.
//!
//! Unlike framework-layer conventions (e.g. `MENTIONS` / `PART_OF` edges), the
//! provenance graph is an engine structure. This enables: results that are
//! citable by construction, source-level access filtering, and cascading
//! invalidation when a source document is retracted.

use chronos_common::{ChunkId, DocId, EdgeId, ProvenanceRef, Result};

/// Maintains the `triple <-> chunk <-> document` links.
pub trait ProvenanceStore: Send + Sync {
    /// Record that `fact` was extracted from `source`.
    fn link(&mut self, fact: EdgeId, source: ProvenanceRef) -> Result<()>;

    /// All facts derived from a given chunk.
    fn facts_from_chunk(&self, chunk: ChunkId) -> Result<Vec<EdgeId>>;

    /// All facts derived from a given document.
    fn facts_from_doc(&self, doc: DocId) -> Result<Vec<EdgeId>>;

    /// Facts that should be invalidated when `doc` is retracted (cascade).
    fn cascade_on_retract(&self, doc: DocId) -> Result<Vec<EdgeId>>;
}

/// A citation emitted alongside retrieved context.
#[derive(Debug, Clone)]
pub struct Citation {
    pub source: ProvenanceRef,
    pub snippet: Option<String>,
}
