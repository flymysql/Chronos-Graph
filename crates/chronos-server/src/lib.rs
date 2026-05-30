//! Service layer.
//!
//! Hosts the engine behind gRPC/HTTP, manages sessions, and enforces
//! multi-tenant access control. Networking (tonic/axum) and the async runtime
//! (tokio) are planned; the skeleton defines the request surface.

pub mod acl;
pub mod session;

use chronos_common::{AsOf, Result, TokenBudget};
use chronos_query::ContextBlock;

/// A retrieval request as received over the wire.
#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub tenant: u64,
    pub query: String,
    pub budget: TokenBudget,
    pub at: AsOf,
}

/// The engine-facing service interface that transports (gRPC/REST) call into.
pub trait ChronosService: Send + Sync {
    fn search(&self, req: SearchRequest) -> Result<ContextBlock>;
}
