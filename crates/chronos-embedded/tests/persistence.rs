//! M6: durability. Facts, names, tenants and derived indexes survive a process
//! restart when the store is backed by RocksDB.
#![cfg(feature = "rocks")]

use chronos_common::{AsOf, ChunkId, DocId, TenantId, Timestamp};
use chronos_embedded::{FactStore, MemoryRetriever};
use chronos_temporal::ConflictPolicy;

fn temp_path(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("chronos-fs-{tag}-{nanos}"));
    p
}

#[test]
fn facts_names_and_tenants_survive_reopen() {
    let path = temp_path("reopen");
    let t1 = TenantId::new(7);

    {
        let store = FactStore::open_rocks(&path).unwrap();
        store
            .ingest_for(
                t1,
                "Alice",
                "lives_in",
                "Shanghai",
                Timestamp::from_millis(2_000),
                DocId::new(20),
                ChunkId::new(1),
                ConflictPolicy::UniqueSubjectPredicate,
            )
            .unwrap();
        // Default-tenant fact too, to check tenant isolation survives.
        store
            .ingest(
                "Bob",
                "works_at",
                "Acme",
                Timestamp::from_millis(1_000),
                DocId::new(30),
                ChunkId::new(1),
                ConflictPolicy::UniqueSubjectPredicate,
            )
            .unwrap();
    } // store dropped: RocksDB closed.

    // Reopen from disk and verify everything recovered.
    {
        let store = FactStore::open_rocks(&path).unwrap();

        // Tenant-scoped retrieval recovers facts, names and provenance.
        let r1 = MemoryRetriever::new_for(&store, t1)
            .answer("MATCH (n) WHERE SIMILAR(n, \"Alice\") RETURN CONTEXT(cite = true)")
            .unwrap();
        assert!(r1.text.contains("Shanghai"), "got: {}", r1.text);
        assert_eq!(r1.citations[0].source.doc, DocId::new(20));

        // Tenant isolation persisted: default tenant doesn't see tenant 7's fact.
        assert!(store
            .as_of(AsOf::now())
            .unwrap()
            .facts
            .iter()
            .all(|f| { store.node_name(f.subject) != "Alice" }));
        assert_eq!(store.as_of_for(t1, AsOf::now()).unwrap().facts.len(), 1);

        // New ingest does not collide with recovered edge ids.
        let edge = store
            .ingest(
                "Carol",
                "knows",
                "Dave",
                Timestamp::from_millis(3_000),
                DocId::new(40),
                ChunkId::new(1),
                ConflictPolicy::AppendOnly,
            )
            .unwrap();
        assert!(edge.raw() >= 3);

        // Communities recovered for the default tenant (Bob/Acme + Carol/Dave).
        let comms = store.community_summaries().unwrap();
        assert!(comms.iter().any(|c| c.summary.contains("Acme")));
    }

    let _ = std::fs::remove_dir_all(&path);
}
