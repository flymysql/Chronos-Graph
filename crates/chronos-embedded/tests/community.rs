//! M4: connected-component communities are maintained incrementally as facts
//! are ingested, and summaries reflect the *current* (valid-now) facts.

use chronos_common::{ChunkId, DocId, Timestamp};
use chronos_embedded::FactStore;
use chronos_temporal::ConflictPolicy;

fn ingest(store: &FactStore, s: &str, p: &str, o: &str, vf: i64, doc: u64) {
    store
        .ingest(
            s,
            p,
            o,
            Timestamp::from_millis(vf),
            DocId::new(doc),
            ChunkId::new(1),
            ConflictPolicy::AppendOnly,
        )
        .unwrap();
}

#[test]
fn disjoint_clusters_form_separate_communities() {
    let store = FactStore::new();
    ingest(&store, "Alice", "lives_in", "Beijing", 1_000, 10);
    ingest(&store, "Bob", "lives_in", "Tokyo", 1_000, 20);

    let comms = store.community_summaries().unwrap();
    assert_eq!(comms.len(), 2);

    // A bridging fact merges the two clusters into one.
    ingest(&store, "Alice", "knows", "Bob", 1_500, 30);
    let comms = store.community_summaries().unwrap();
    assert_eq!(comms.len(), 1);
    let summary = &comms[0].summary;
    for name in ["Alice", "Bob", "Beijing", "Tokyo"] {
        assert!(summary.contains(name), "summary missing {name}: {summary}");
    }
}

#[test]
fn summary_reflects_only_currently_valid_facts() {
    let store = FactStore::new();
    // Supersede Beijing with Shanghai under unique-subject-predicate policy.
    ingest(&store, "Alice", "lives_in", "Beijing", 1_000, 10);
    ingest_unique(&store, "Alice", "lives_in", "Shanghai", 2_000, 20);

    let comms = store.community_summaries().unwrap();
    // Alice, Beijing and Shanghai are all in one component (connected via Alice),
    // but the summary's current facts only mention Shanghai.
    let summary = &comms[0].summary;
    assert!(summary.contains("Shanghai"), "got: {summary}");
    assert!(
        !summary.contains("lives_in Beijing"),
        "stale fact leaked: {summary}"
    );
}

fn ingest_unique(store: &FactStore, s: &str, p: &str, o: &str, vf: i64, doc: u64) {
    store
        .ingest(
            s,
            p,
            o,
            Timestamp::from_millis(vf),
            DocId::new(doc),
            ChunkId::new(1),
            ConflictPolicy::UniqueSubjectPredicate,
        )
        .unwrap();
}
