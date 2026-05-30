//! M2 end-to-end: a natural-ish query string compiles, retrieves over the
//! bitemporal FactStore, and produces cited, point-in-time-correct context.

use chronos_common::{ChunkId, DocId, NodeId, PredicateId, ProvenanceRef, Timestamp};
use chronos_embedded::{FactStore, MemoryRetriever};
use chronos_temporal::{BitemporalSpan, ConflictPolicy, Fact};

const ALICE: NodeId = NodeId(1);
const BEIJING: NodeId = NodeId(100);
const SHANGHAI: NodeId = NodeId(200);
const LIVES_IN: PredicateId = PredicateId(1);

fn seed() -> FactStore {
    let store = FactStore::new();
    store.put_node(ALICE, "Alice");
    store.put_node(BEIJING, "Beijing");
    store.put_node(SHANGHAI, "Shanghai");
    store.put_predicate(LIVES_IN, "lives_in");

    let mk = |store: &FactStore, object: NodeId, vf: i64, doc: u64| Fact {
        id: store.next_edge_id(),
        subject: ALICE,
        predicate: LIVES_IN,
        object,
        span: BitemporalSpan::open(Timestamp::from_millis(vf), Timestamp::MIN),
        provenance: ProvenanceRef::new(DocId::new(doc), ChunkId::new(1)),
        embedding: None,
    };

    store
        .upsert_fact(
            mk(&store, BEIJING, 1_000, 10),
            ConflictPolicy::UniqueSubjectPredicate,
        )
        .unwrap();
    store
        .upsert_fact(
            mk(&store, SHANGHAI, 2_000, 20),
            ConflictPolicy::UniqueSubjectPredicate,
        )
        .unwrap();
    store
}

#[test]
fn question_to_cited_context_current() {
    let store = seed();
    let retriever = MemoryRetriever::new(&store);

    let block = retriever
        .answer("MATCH (n) WHERE SIMILAR(n, \"Alice lives\") RETURN CONTEXT(cite = true)")
        .unwrap();

    // Current view: Alice lives in Shanghai, with a citation to its source doc.
    assert!(block.text.contains("Shanghai"), "got: {}", block.text);
    assert!(!block.text.contains("Beijing"), "got: {}", block.text);
    assert_eq!(block.citations.len(), 1);
    assert_eq!(block.citations[0].source.doc, DocId::new(20));
}

#[test]
fn point_in_time_query_returns_historical_fact() {
    let store = seed();
    let retriever = MemoryRetriever::new(&store);

    // As of valid time 1500 (between the two), Alice lived in Beijing.
    let block = retriever
        .answer(
            "MATCH (n) WHERE SIMILAR(n, \"Alice\") AS OF VALID TIME 1500 \
             RETURN CONTEXT(cite = true)",
        )
        .unwrap();

    assert!(block.text.contains("Beijing"), "got: {}", block.text);
    assert!(!block.text.contains("Shanghai"), "got: {}", block.text);
    assert_eq!(block.citations[0].source.doc, DocId::new(10));
}

#[test]
fn budget_limits_returned_lines() {
    let store = seed();
    // Add a second subject so two facts are currently valid.
    store.put_node(NodeId::new(2), "Bob");
    store.put_node(NodeId::new(300), "Tokyo");
    store.put_predicate(PredicateId::new(2), "works_in");
    let f = Fact {
        id: store.next_edge_id(),
        subject: NodeId::new(2),
        predicate: PredicateId::new(2),
        object: NodeId::new(300),
        span: BitemporalSpan::open(Timestamp::from_millis(500), Timestamp::MIN),
        provenance: ProvenanceRef::new(DocId::new(30), ChunkId::new(1)),
        embedding: None,
    };
    store.upsert_fact(f, ConflictPolicy::AppendOnly).unwrap();

    let retriever = MemoryRetriever::new(&store);
    // Budget of 3 tokens fits exactly one "subject predicate object" line.
    let block = retriever
        .answer("MATCH (n) TRAVERSE SEMANTIC(budget = 3 tokens) RETURN CONTEXT(cite = false)")
        .unwrap();
    assert_eq!(block.text.lines().count(), 1, "got: {}", block.text);
}
