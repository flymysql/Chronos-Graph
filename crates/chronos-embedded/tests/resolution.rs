//! M4: entity resolution merges surface variants into one canonical node,
//! rewriting their facts while preserving spans/provenance, and deduping the
//! exact duplicates the merge creates.

use chronos_common::{AsOf, ChunkId, DocId, Timestamp};
use chronos_embedded::{FactStore, MemoryRetriever};
use chronos_temporal::ConflictPolicy;

fn ingest(store: &FactStore, s: &str, p: &str, o: &str, doc: u64) {
    store
        .ingest(
            s,
            p,
            o,
            Timestamp::from_millis(1_000),
            DocId::new(doc),
            ChunkId::new(1),
            ConflictPolicy::AppendOnly,
        )
        .unwrap();
}

#[test]
fn candidate_detection_finds_surface_variants() {
    let store = FactStore::new();
    ingest(&store, "OpenAI Inc.", "based_in", "SF", 10);
    ingest(&store, "OpenAI", "hires", "Researchers", 20);
    ingest(&store, "Beijing", "capital_of", "China", 30);

    let cands = store.resolution_candidates(0.9);
    assert_eq!(cands.len(), 1, "got: {cands:?}");
    let (keep, drop, score) = cands[0];
    assert_eq!(score, 1.0);
    // Both surface forms participate; lower id is the survivor.
    assert!(keep.raw() < drop.raw());
}

#[test]
fn merge_rewrites_facts_to_canonical_node() {
    let store = FactStore::new();
    ingest(&store, "OpenAI Inc.", "based_in", "SF", 10);
    ingest(&store, "OpenAI", "hires", "Researchers", 20);

    let merged = store.auto_resolve(0.9).unwrap();
    assert_eq!(merged, 1);

    // Both facts now hang off the same subject node.
    let facts = store.as_of(AsOf::now()).unwrap().facts;
    assert_eq!(facts.len(), 2);
    let subjects: std::collections::BTreeSet<_> = facts.iter().map(|f| f.subject).collect();
    assert_eq!(subjects.len(), 1, "facts not unified onto one node");

    // Retrieval over the canonical entity surfaces both facts with citations.
    let retriever = MemoryRetriever::new(&store);
    let block = retriever
        .answer("MATCH (n) WHERE SIMILAR(n, \"OpenAI\") RETURN CONTEXT(cite = true)")
        .unwrap();
    assert!(block.text.contains("SF"), "got: {}", block.text);
    assert!(block.text.contains("Researchers"), "got: {}", block.text);
}

#[test]
fn merge_dedupes_exact_duplicate_facts() {
    let store = FactStore::new();
    // Two sources assert the same fact under different surface names.
    ingest(&store, "OpenAI Inc.", "based_in", "SF", 10);
    ingest(&store, "OpenAI", "based_in", "SF", 20);

    assert_eq!(store.auto_resolve(0.9).unwrap(), 1);

    // After merge the two facts collapse into one (same s/p/o/valid_from).
    let facts = store.as_of(AsOf::now()).unwrap().facts;
    assert_eq!(facts.len(), 1, "duplicate not deduped: {facts:?}");
}
