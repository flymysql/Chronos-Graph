//! M4: tenant isolation. Facts, point-in-time views, communities and entity
//! resolution are all scoped to a tenant; one tenant never observes another's
//! data, and contradiction detection never crosses the boundary.

use chronos_common::{AsOf, ChunkId, DocId, TenantId, Timestamp};
use chronos_embedded::{FactStore, MemoryRetriever};
use chronos_temporal::ConflictPolicy;

const T1: TenantId = TenantId(1);
const T2: TenantId = TenantId(2);

fn ingest(store: &FactStore, t: TenantId, s: &str, p: &str, o: &str, vf: i64, doc: u64) {
    store
        .ingest_for(
            t,
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

#[test]
fn facts_are_isolated_per_tenant() {
    let store = FactStore::new();
    ingest(&store, T1, "Alice", "lives_in", "Beijing", 1_000, 10);
    ingest(&store, T2, "Alice", "lives_in", "Tokyo", 1_000, 20);

    let t1 = store.as_of_for(T1, AsOf::now()).unwrap().facts;
    let t2 = store.as_of_for(T2, AsOf::now()).unwrap().facts;
    assert_eq!(t1.len(), 1);
    assert_eq!(t2.len(), 1);

    // The DEFAULT tenant sees neither.
    assert!(store.as_of(AsOf::now()).unwrap().facts.is_empty());

    // Retrieval is scoped: tenant 1 sees Beijing, tenant 2 sees Tokyo.
    let r1 = MemoryRetriever::new_for(&store, T1)
        .answer("MATCH (n) WHERE SIMILAR(n, \"Alice\") RETURN CONTEXT(cite = true)")
        .unwrap();
    assert!(
        r1.text.contains("Beijing") && !r1.text.contains("Tokyo"),
        "{}",
        r1.text
    );

    let r2 = MemoryRetriever::new_for(&store, T2)
        .answer("MATCH (n) WHERE SIMILAR(n, \"Alice\") RETURN CONTEXT(cite = true)")
        .unwrap();
    assert!(
        r2.text.contains("Tokyo") && !r2.text.contains("Beijing"),
        "{}",
        r2.text
    );
}

#[test]
fn contradiction_detection_does_not_cross_tenants() {
    let store = FactStore::new();
    // Same subject/predicate in two tenants under UniqueSubjectPredicate.
    ingest(&store, T1, "Alice", "lives_in", "Beijing", 1_000, 10);
    ingest(&store, T2, "Alice", "lives_in", "Tokyo", 2_000, 20);
    // Tenant 1's fact must remain open (not superseded by tenant 2's write).
    let t1 = store.as_of_for(T1, AsOf::now()).unwrap().facts;
    assert_eq!(t1.len(), 1);
    assert_eq!(store.node_name(t1[0].object), "Beijing");
}

#[test]
fn communities_are_scoped_to_tenant() {
    let store = FactStore::new();
    ingest(&store, T1, "Alice", "knows", "Bob", 1_000, 10);
    ingest(&store, T2, "Carol", "knows", "Dave", 1_000, 20);

    let c1 = store.community_summaries_for(T1).unwrap();
    let c2 = store.community_summaries_for(T2).unwrap();
    assert_eq!(c1.len(), 1);
    assert_eq!(c2.len(), 1);
    assert!(c1[0].summary.contains("Alice") && !c1[0].summary.contains("Carol"));
    assert!(c2[0].summary.contains("Carol") && !c2[0].summary.contains("Alice"));
}

#[test]
fn resolution_is_scoped_to_tenant() {
    let store = FactStore::new();
    // Tenant 1 has the variant pair; tenant 2 does not.
    ingest(&store, T1, "OpenAI Inc.", "based_in", "SF", 1_000, 10);
    ingest(&store, T1, "OpenAI", "hires", "Researchers", 1_000, 11);
    ingest(&store, T2, "Beijing", "capital_of", "China", 1_000, 20);

    assert_eq!(store.resolution_candidates_for(T1, 0.9).len(), 1);
    assert_eq!(store.resolution_candidates_for(T2, 0.9).len(), 0);

    assert_eq!(store.auto_resolve_for(T1, 0.9).unwrap(), 1);
    // Tenant 1's facts are unified; tenant 2 is untouched.
    let t1 = store.as_of_for(T1, AsOf::now()).unwrap().facts;
    let subjects: std::collections::BTreeSet<_> = t1.iter().map(|f| f.subject).collect();
    assert_eq!(subjects.len(), 1);
    assert_eq!(store.as_of_for(T2, AsOf::now()).unwrap().facts.len(), 1);
}
