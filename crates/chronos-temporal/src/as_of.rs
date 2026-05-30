//! Point-in-time (`as-of`) evaluation against both timelines.

use crate::fact::{Fact, FactView};
use chronos_common::AsOf;

/// Resolves the set of facts visible at a chosen bitemporal coordinate.
pub trait AsOfResolver {
    /// Materialize the fact view visible at `at`.
    fn resolve(&self, at: AsOf) -> FactView;
}

/// A simple in-memory resolver, useful for tests and the embedded form factor.
pub struct InMemoryResolver {
    pub facts: Vec<Fact>,
}

impl AsOfResolver for InMemoryResolver {
    fn resolve(&self, at: AsOf) -> FactView {
        FactView {
            facts: self
                .facts
                .iter()
                .filter(|f| f.span.visible_at(at))
                .cloned()
                .collect(),
        }
    }
}
