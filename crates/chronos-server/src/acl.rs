//! [Phase 2] Access-control push-down.
//!
//! Permission predicates are pushed into traversal so that retrieval never
//! "sees" nodes the principal may not access — rather than filtering after the
//! fact.

use crate::session::Session;
use chronos_common::NodeId;

pub trait AccessControl: Send + Sync {
    /// Whether `session` may read `node`. Intended to be lowered into the scan.
    fn can_read(&self, session: &Session, node: NodeId) -> bool;
}
