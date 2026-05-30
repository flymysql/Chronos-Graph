//! Bitemporal interval index.
//!
//! Lets the engine locate facts active at a `(valid_t, tx_t)` coordinate via a
//! range scan instead of a full-table property filter — the key enabler of
//! fast point-in-time queries.

use chronos_common::{AsOf, BitemporalSpan, EdgeId, Result, Timestamp};

pub trait IntervalIndex: Send + Sync {
    /// Index a newly written fact's span.
    fn insert(&mut self, id: EdgeId, span: &BitemporalSpan) -> Result<()>;

    /// Close a fact's transaction-time bound (invalidation).
    fn close(&mut self, id: EdgeId, at: Timestamp) -> Result<()>;

    /// Edge ids active at the given bitemporal coordinate.
    fn query_active(&self, at: AsOf) -> Result<Vec<EdgeId>>;
}

/// A straightforward in-memory implementation (linear scan). Production uses an
/// interval tree / time-partitioned segments; this keeps the skeleton honest
/// and testable.
#[derive(Default)]
pub struct InMemoryIntervalIndex {
    entries: Vec<(EdgeId, BitemporalSpan)>,
}

impl IntervalIndex for InMemoryIntervalIndex {
    fn insert(&mut self, id: EdgeId, span: &BitemporalSpan) -> Result<()> {
        self.entries.push((id, *span));
        Ok(())
    }

    fn close(&mut self, id: EdgeId, at: Timestamp) -> Result<()> {
        for (eid, span) in self.entries.iter_mut() {
            if *eid == id && span.tx_to.is_none() {
                span.close_tx(at);
            }
        }
        Ok(())
    }

    fn query_active(&self, at: AsOf) -> Result<Vec<EdgeId>> {
        Ok(self
            .entries
            .iter()
            .filter(|(_, span)| span.visible_at(at))
            .map(|(id, _)| *id)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_hides_fact_from_current_view() {
        let mut idx = InMemoryIntervalIndex::default();
        let span = BitemporalSpan::open(Timestamp::from_millis(0), Timestamp::from_millis(0));
        idx.insert(EdgeId::new(1), &span).unwrap();
        assert_eq!(idx.query_active(AsOf::now()).unwrap(), vec![EdgeId::new(1)]);

        idx.close(EdgeId::new(1), Timestamp::from_millis(10))
            .unwrap();
        assert!(idx.query_active(AsOf::now()).unwrap().is_empty());
    }
}
