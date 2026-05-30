//! Strongly-typed identifiers. Newtypes prevent mixing up id kinds.

macro_rules! id_newtype {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u64);

        impl $name {
            pub const fn new(v: u64) -> Self {
                Self(v)
            }
            pub const fn raw(self) -> u64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

id_newtype!(
    /// Identifies a graph node (an entity).
    NodeId
);
id_newtype!(
    /// Identifies a graph edge (a fact / relationship).
    EdgeId
);
id_newtype!(
    /// Identifies a predicate (relationship type).
    PredicateId
);
id_newtype!(
    /// Identifies a stored embedding vector.
    VectorId
);
id_newtype!(
    /// Identifies a source document.
    DocId
);
id_newtype!(
    /// Identifies a source chunk within a document.
    ChunkId
);
id_newtype!(
    /// Identifies a tenant (isolation boundary). Facts, retrieval, communities
    /// and entity resolution are all scoped to a tenant. `TenantId::DEFAULT`
    /// (0) is the single-tenant / system tenant used when none is specified.
    TenantId
);

impl TenantId {
    /// The default (single-tenant) boundary.
    pub const DEFAULT: TenantId = TenantId(0);
}

/// A back-reference from a fact to the source text it was extracted from.
///
/// Provenance is a first-class structure in Chronos-Graph (see
/// `chronos-provenance`): every fact can be traced `triple -> chunk -> document`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProvenanceRef {
    pub doc: DocId,
    pub chunk: ChunkId,
}

impl ProvenanceRef {
    pub const fn new(doc: DocId, chunk: ChunkId) -> Self {
        Self { doc, chunk }
    }
}
