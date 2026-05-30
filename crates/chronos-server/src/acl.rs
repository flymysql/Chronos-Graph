//! Access-control / multi-tenancy push-down.
//!
//! Tenancy is the first isolation boundary and is enforced **inside the
//! engine**: every fact carries a [`TenantId`], and retrieval, community
//! summaries and entity resolution all push a tenant filter into the scan
//! (`FactStore::as_of_for`, `community_summaries_for`, `auto_resolve_for`).
//! A retriever scoped to one tenant can never observe another tenant's facts,
//! rather than filtering after the fact. The HTTP layer derives the tenant
//! from the `X-Tenant-Id` header (see [`crate::tenant_from`]).
//!
//! Finer-grained, per-node permission push-down (below the tenant boundary) is
//! future work and is sketched by the [`AccessControl`] trait.

use crate::session::Session;
use chronos_common::{NodeId, TenantId};

/// Map a session to its tenant boundary.
pub fn session_tenant(session: &Session) -> TenantId {
    TenantId::new(session.tenant)
}

pub trait AccessControl: Send + Sync {
    /// Whether `session` may read `node`. Intended to be lowered into the scan.
    fn can_read(&self, session: &Session, node: NodeId) -> bool;
}
