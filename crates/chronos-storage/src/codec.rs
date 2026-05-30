//! Record encoding for bitemporal facts and nodes.
//!
//! The key layout is designed so that an `IntervalIndex` can range-scan active
//! facts without a full property filter. A sketch of the planned key:
//!
//! ```text
//!   [tag:1][subject:8][predicate:8][valid_from:8][tx_from:8] -> value
//! ```

use chronos_common::{BitemporalSpan, EdgeId};

/// Encode a fact's primary key (placeholder layout for the skeleton).
pub fn encode_fact_key(edge: EdgeId, span: &BitemporalSpan) -> Vec<u8> {
    let mut key = Vec::with_capacity(1 + 8 + 8 + 8);
    key.push(b'F');
    key.extend_from_slice(&edge.raw().to_be_bytes());
    key.extend_from_slice(&span.valid_from.millis().to_be_bytes());
    key.extend_from_slice(&span.tx_from.millis().to_be_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_common::Timestamp;

    #[test]
    fn fact_key_is_prefixed_and_sized() {
        let span = BitemporalSpan::open(Timestamp::from_millis(1), Timestamp::from_millis(2));
        let key = encode_fact_key(EdgeId::new(7), &span);
        assert_eq!(key[0], b'F');
        assert_eq!(key.len(), 1 + 8 + 8 + 8);
    }
}
