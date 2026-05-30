//! Time and the bitemporal model.
//!
//! Chronos-Graph tracks two independent timelines (see `docs/design.md`):
//! - **valid time**: when a fact was true in the real world.
//! - **transaction time**: when the system learned about the fact.

/// Milliseconds since the Unix epoch. A monotonic logical clock may also be
/// layered on top for total ordering of transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub const MIN: Timestamp = Timestamp(i64::MIN);
    pub const MAX: Timestamp = Timestamp(i64::MAX);

    pub const fn from_millis(ms: i64) -> Self {
        Self(ms)
    }
    pub const fn millis(self) -> i64 {
        self.0
    }
}

/// A bitemporal validity window over both timelines.
///
/// A `None` upper bound means "still open" (valid until now / current version).
/// Invalidation closes a span by setting its upper bounds rather than deleting
/// the record, preserving full history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitemporalSpan {
    pub valid_from: Timestamp,
    pub valid_to: Option<Timestamp>,
    pub tx_from: Timestamp,
    pub tx_to: Option<Timestamp>,
}

impl BitemporalSpan {
    /// A span that is open on both timelines starting at the given instants.
    pub const fn open(valid_from: Timestamp, tx_from: Timestamp) -> Self {
        Self {
            valid_from,
            valid_to: None,
            tx_from,
            tx_to: None,
        }
    }

    /// Whether this span is visible at the given point-in-time selector.
    pub fn visible_at(&self, at: AsOf) -> bool {
        let valid_ok =
            at.valid_time >= self.valid_from && self.valid_to.is_none_or(|hi| at.valid_time < hi);
        let tx_ok = at.tx_time >= self.tx_from && self.tx_to.is_none_or(|hi| at.tx_time < hi);
        valid_ok && tx_ok
    }

    /// Close the transaction-time bound (logical "expire" of this version).
    pub fn close_tx(&mut self, at: Timestamp) {
        self.tx_to = Some(at);
    }

    /// Close the valid-time bound (the fact stopped being true in the world).
    pub fn close_valid(&mut self, at: Timestamp) {
        self.valid_to = Some(at);
    }
}

/// Point-in-time selector for queries: pick a position on each timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsOf {
    pub valid_time: Timestamp,
    pub tx_time: Timestamp,
}

impl AsOf {
    pub const fn new(valid_time: Timestamp, tx_time: Timestamp) -> Self {
        Self {
            valid_time,
            tx_time,
        }
    }

    /// "Now" on both timelines (the default current-state view).
    pub const fn now() -> Self {
        Self {
            valid_time: Timestamp::MAX,
            tx_time: Timestamp::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_span_is_visible_now() {
        let span = BitemporalSpan::open(Timestamp::from_millis(0), Timestamp::from_millis(0));
        assert!(span.visible_at(AsOf::now()));
    }

    #[test]
    fn closed_valid_span_hidden_after_end() {
        let mut span = BitemporalSpan::open(Timestamp::from_millis(0), Timestamp::from_millis(0));
        span.close_valid(Timestamp::from_millis(100));
        let at = AsOf::new(Timestamp::from_millis(200), Timestamp::MAX);
        assert!(!span.visible_at(at));
        let before = AsOf::new(Timestamp::from_millis(50), Timestamp::MAX);
        assert!(span.visible_at(before));
    }
}
