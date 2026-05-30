//! M1 correctness tests for the transactional FactStore: bitemporal semantics,
//! point-in-time queries, and the non-lossy invalidation invariant.

use chronos_common::{
    AsOf, BitemporalSpan, ChunkId, DocId, EdgeId, NodeId, PredicateId, ProvenanceRef, Timestamp,
};
use chronos_embedded::FactStore;
use chronos_temporal::{ConflictPolicy, Fact};
use proptest::prelude::*;

const SUBJ: NodeId = NodeId(1);
const PRED: PredicateId = PredicateId(1);

fn fact(store: &FactStore, object: u64, valid_from: i64) -> Fact {
    Fact {
        id: store.next_edge_id(),
        subject: SUBJ,
        predicate: PRED,
        object: NodeId::new(object),
        // tx_from is overwritten by the engine; valid_to is open until superseded.
        span: BitemporalSpan::open(Timestamp::from_millis(valid_from), Timestamp::MIN),
        provenance: ProvenanceRef::new(DocId::new(1), ChunkId::new(object)),
        embedding: None,
    }
}

fn open_facts_for(store: &FactStore, at: AsOf) -> Vec<Fact> {
    store
        .as_of(at)
        .unwrap()
        .facts
        .into_iter()
        .filter(|f| f.subject == SUBJ && f.predicate == PRED)
        .collect()
}

#[test]
fn moved_cities_scenario() {
    // The design-doc example: user lived in "Beijing" then moved to "Shanghai".
    let store = FactStore::new();
    let beijing = 100; // object id standing in for "Beijing"
    let shanghai = 200; // object id standing in for "Shanghai"

    store
        .upsert_fact(
            fact(&store, beijing, 1_000),
            ConflictPolicy::UniqueSubjectPredicate,
        )
        .unwrap();
    let out = store
        .upsert_fact(
            fact(&store, shanghai, 2_000),
            ConflictPolicy::UniqueSubjectPredicate,
        )
        .unwrap();

    // Supersession invalidated exactly one prior fact (valid-time closed).
    assert_eq!(out.invalidated.len(), 1);
    assert_eq!(out.invalidated[0].object, NodeId::new(beijing));
    assert_eq!(
        out.invalidated[0].span.valid_to,
        Some(Timestamp::from_millis(2_000))
    );

    // Now: exactly one current fact, and it is Shanghai.
    let now = open_facts_for(&store, AsOf::now());
    assert_eq!(now.len(), 1);
    assert_eq!(now[0].object, NodeId::new(shanghai));

    // Point-in-time at valid=1_500 (between the two): still Beijing.
    let past = AsOf::new(Timestamp::from_millis(1_500), Timestamp::MAX);
    let past_facts = open_facts_for(&store, past);
    assert_eq!(past_facts.len(), 1);
    assert_eq!(past_facts[0].object, NodeId::new(beijing));

    // Non-lossy: both versions are retained.
    assert_eq!(store.all_facts().unwrap().len(), 2);
}

#[test]
fn retraction_is_transaction_time_scoped() {
    let store = FactStore::new();
    let out = store
        .upsert_fact(fact(&store, 1, 10), ConflictPolicy::AppendOnly)
        .unwrap();
    let id = out.written.id;
    let learned_tx = out.written.span.tx_from;

    // Retract (recording error) at a transaction time strictly after tx_from.
    let retract_at = Timestamp::from_millis(learned_tx.millis() + 1);
    store.retract_fact(id, retract_at).unwrap();

    // Hidden under current knowledge...
    assert!(open_facts_for(&store, AsOf::now()).is_empty());
    // ...but still visible as of an earlier transaction time (audit).
    let before = AsOf::new(Timestamp::MAX, learned_tx);
    assert_eq!(open_facts_for(&store, before).len(), 1);
}

proptest! {
    /// After a chain of supersessions with strictly increasing valid_from:
    /// exactly one fact is currently valid, and a point-in-time query inside
    /// window i returns the i-th object. History is fully preserved.
    #[test]
    fn supersession_chain_is_consistent_and_nonlossy(gaps in prop::collection::vec(1i64..50, 1..8)) {
        let store = FactStore::new();

        // Build strictly increasing valid_from instants.
        let mut valid_from = Vec::new();
        let mut acc = 1_000i64;
        for g in &gaps {
            valid_from.push(acc);
            acc += g;
        }
        let n = valid_from.len();

        for (i, vf) in valid_from.iter().enumerate() {
            store
                .upsert_fact(
                    fact(&store, (i + 1) as u64, *vf),
                    ConflictPolicy::UniqueSubjectPredicate,
                )
                .unwrap();
        }

        // Exactly one currently-valid fact, equal to the last inserted.
        let current = open_facts_for(&store, AsOf::now());
        prop_assert_eq!(current.len(), 1);
        prop_assert_eq!(current[0].object, NodeId::new(n as u64));

        // Non-lossy: every version retained.
        prop_assert_eq!(store.all_facts().unwrap().len(), n);

        // Point-in-time inside each window returns that window's object.
        for (i, vf) in valid_from.iter().enumerate() {
            let at = AsOf::new(Timestamp::from_millis(*vf), Timestamp::MAX);
            let f = open_facts_for(&store, at);
            prop_assert_eq!(f.len(), 1);
            prop_assert_eq!(f[0].object, NodeId::new((i + 1) as u64));
        }

        // The interval-index accelerator agrees with the authoritative scan.
        let mut idx_ids: Vec<EdgeId> = store.active_via_index(AsOf::now()).unwrap();
        let mut scan_ids: Vec<EdgeId> =
            store.as_of(AsOf::now()).unwrap().facts.iter().map(|f| f.id).collect();
        idx_ids.sort();
        scan_ids.sort();
        prop_assert_eq!(idx_ids, scan_ids);
    }
}
