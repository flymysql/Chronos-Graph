//! `FactStore`: the transactional heart of the M1 engine.
//!
//! Ties the storage engine, the bitemporal interval index and provenance into
//! one unit, and implements the two defining operations:
//!
//! - [`FactStore::upsert_fact`] — the atomic `UPSERT_FACT` operator: within a
//!   single transaction, detect contradicting facts, close their valid-time
//!   span (real-world supersession, *not* deletion), and append the new fact.
//! - [`FactStore::as_of`] — point-in-time query over both timelines.

use crate::fact_codec::{decode_fact, encode_fact, fact_key, FACT_PREFIX};
use chronos_common::{
    AsOf, BitemporalSpan, ChunkId, DocId, EdgeId, NodeId, PredicateId, ProvenanceRef, Result,
    Timestamp,
};
use chronos_community::InMemoryCommunityIndex;
use chronos_resolution::LexicalBlocker;
use chronos_storage::{
    InMemoryIntervalIndex, IntervalIndex, KeyRange, MemoryEngine, StorageEngine,
};
use chronos_temporal::invalidation::facts_to_invalidate;
use chronos_temporal::{ConflictPolicy, Fact, FactView, UpsertOutcome};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

pub struct FactStore {
    engine: MemoryEngine,
    index: RwLock<InMemoryIntervalIndex>,
    provenance: RwLock<HashMap<EdgeId, ProvenanceRef>>,
    nodes: RwLock<HashMap<NodeId, String>>,
    predicates: RwLock<HashMap<PredicateId, String>>,
    node_ids: RwLock<HashMap<String, NodeId>>,
    predicate_ids: RwLock<HashMap<String, PredicateId>>,
    communities: RwLock<InMemoryCommunityIndex>,
    next_edge: AtomicU64,
    next_node: AtomicU64,
    next_predicate: AtomicU64,
}

/// A community with resolved member names and a templated summary.
#[derive(Debug, Clone)]
pub struct CommunitySummary {
    pub id: u64,
    pub members: Vec<String>,
    pub summary: String,
}

impl Default for FactStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FactStore {
    pub fn new() -> Self {
        Self {
            engine: MemoryEngine::new(),
            index: RwLock::new(InMemoryIntervalIndex::default()),
            provenance: RwLock::new(HashMap::new()),
            nodes: RwLock::new(HashMap::new()),
            predicates: RwLock::new(HashMap::new()),
            node_ids: RwLock::new(HashMap::new()),
            predicate_ids: RwLock::new(HashMap::new()),
            communities: RwLock::new(InMemoryCommunityIndex::new()),
            next_edge: AtomicU64::new(1),
            next_node: AtomicU64::new(1),
            next_predicate: AtomicU64::new(1),
        }
    }

    /// Level-0 communities (connected components) with resolved member names
    /// and a templated summary built from the facts currently valid among
    /// their members. This is the "global" view used to answer topic-level
    /// questions.
    pub fn community_summaries(&self) -> Result<Vec<CommunitySummary>> {
        let comms = self
            .communities
            .read()
            .expect("communities poisoned")
            .communities();
        let current = self.as_of(AsOf::now())?.facts;
        let mut out = Vec::with_capacity(comms.len());
        for c in comms {
            let member_set: std::collections::BTreeSet<NodeId> =
                c.members.iter().copied().collect();
            let member_names: Vec<String> = c.members.iter().map(|n| self.node_name(*n)).collect();
            let facts: Vec<String> = current
                .iter()
                .filter(|f| member_set.contains(&f.subject))
                .map(|f| self.verbalize(f))
                .collect();
            let summary = format!(
                "Community of {} entities ({}). Current facts: {}",
                member_names.len(),
                member_names.join(", "),
                if facts.is_empty() {
                    "none".to_string()
                } else {
                    facts.join("; ")
                }
            );
            out.push(CommunitySummary {
                id: c.id,
                members: member_names,
                summary,
            });
        }
        Ok(out)
    }

    /// Resolve a node by name, creating and registering it on first use.
    pub fn intern_node(&self, name: &str) -> NodeId {
        if let Some(id) = self.node_ids.read().expect("node_ids poisoned").get(name) {
            return *id;
        }
        let mut ids = self.node_ids.write().expect("node_ids poisoned");
        // Re-check after taking the write lock (another thread may have won).
        if let Some(id) = ids.get(name) {
            return *id;
        }
        let id = NodeId::new(self.next_node.fetch_add(1, Ordering::SeqCst));
        ids.insert(name.to_string(), id);
        self.put_node(id, name);
        id
    }

    /// Resolve a predicate by name, creating and registering it on first use.
    pub fn intern_predicate(&self, name: &str) -> PredicateId {
        if let Some(id) = self
            .predicate_ids
            .read()
            .expect("predicate_ids poisoned")
            .get(name)
        {
            return *id;
        }
        let mut ids = self.predicate_ids.write().expect("predicate_ids poisoned");
        if let Some(id) = ids.get(name) {
            return *id;
        }
        let id = PredicateId::new(self.next_predicate.fetch_add(1, Ordering::SeqCst));
        ids.insert(name.to_string(), id);
        self.put_predicate(id, name);
        id
    }

    /// Allocate a fresh edge id.
    pub fn next_edge_id(&self) -> EdgeId {
        EdgeId::new(self.next_edge.fetch_add(1, Ordering::SeqCst))
    }

    /// Register a human-readable name for a node (used for verbalization and
    /// lexical similarity scoring).
    pub fn put_node(&self, id: NodeId, name: impl Into<String>) {
        self.nodes
            .write()
            .expect("nodes poisoned")
            .insert(id, name.into());
    }

    /// Register a human-readable name for a predicate.
    pub fn put_predicate(&self, id: PredicateId, name: impl Into<String>) {
        self.predicates
            .write()
            .expect("predicates poisoned")
            .insert(id, name.into());
    }

    pub fn node_name(&self, id: NodeId) -> String {
        self.nodes
            .read()
            .expect("nodes poisoned")
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("node#{}", id.raw()))
    }

    pub fn predicate_name(&self, id: PredicateId) -> String {
        self.predicates
            .read()
            .expect("predicates poisoned")
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("rel#{}", id.raw()))
    }

    /// Render a fact as natural-language-ish text: `subject predicate object`.
    pub fn verbalize(&self, fact: &Fact) -> String {
        format!(
            "{} {} {}",
            self.node_name(fact.subject),
            self.predicate_name(fact.predicate),
            self.node_name(fact.object)
        )
    }

    /// Fetch a fact by id at the latest committed state.
    pub fn get_fact(&self, id: EdgeId) -> Result<Option<Fact>> {
        let txn = self.engine.begin()?;
        match self.engine.get(&txn, &fact_key(id))? {
            Some(bytes) => Ok(Some(decode_fact(&bytes)?)),
            None => Ok(None),
        }
    }

    /// All fact records (every version, including invalidated history).
    pub fn all_facts(&self) -> Result<Vec<Fact>> {
        let txn = self.engine.begin()?;
        self.scan_facts(&txn)
    }

    fn scan_facts(&self, txn: &chronos_storage::Txn) -> Result<Vec<Fact>> {
        let mut out = Vec::new();
        for (_k, v) in self.engine.scan(txn, KeyRange::prefix(FACT_PREFIX))? {
            out.push(decode_fact(&v)?);
        }
        Ok(out)
    }

    /// Point-in-time query: facts visible at `at` over both timelines.
    ///
    /// This is the authoritative path (scans the fact key-space). The interval
    /// index is a redundant accelerator validated against it in tests.
    pub fn as_of(&self, at: AsOf) -> Result<FactView> {
        let txn = self.engine.begin()?;
        let facts = self
            .scan_facts(&txn)?
            .into_iter()
            .filter(|f| f.span.visible_at(at))
            .collect();
        Ok(FactView { facts })
    }

    /// Edge ids the interval index reports active at `at` (accelerator path).
    pub fn active_via_index(&self, at: AsOf) -> Result<Vec<EdgeId>> {
        self.index.read().expect("index poisoned").query_active(at)
    }

    pub fn provenance_of(&self, id: EdgeId) -> Option<ProvenanceRef> {
        self.provenance
            .read()
            .expect("prov poisoned")
            .get(&id)
            .copied()
    }

    /// The atomic `UPSERT_FACT` operator.
    ///
    /// `new_fact.span.tx_from` is anchored to the transaction time. Conflicting
    /// facts (per `policy`) have their valid-time closed at the new fact's
    /// `valid_from`, preserving history. All writes commit atomically.
    pub fn upsert_fact(&self, mut new_fact: Fact, policy: ConflictPolicy) -> Result<UpsertOutcome> {
        let mut txn = self.engine.begin()?;
        let now = txn.tx_time;
        new_fact.span.tx_from = now;

        // Candidates: facts currently true (valid-open) and visible now.
        let current_open: Vec<Fact> = self
            .scan_facts(&txn)?
            .into_iter()
            .filter(|f| f.span.valid_to.is_none() && f.span.visible_at(AsOf::now()))
            .collect();

        let to_close = facts_to_invalidate(&new_fact, &current_open, policy, now);

        let mut invalidated = Vec::with_capacity(to_close.len());
        for mut old in to_close {
            // Real-world supersession: the old fact stopped being true when the
            // new one became valid. Close valid-time only; keep it tx-current
            // so historical valid-time queries still return it.
            old.span.close_valid(new_fact.span.valid_from);
            self.engine
                .put(&mut txn, fact_key(old.id), encode_fact(&old))?;
            self.index
                .write()
                .expect("index poisoned")
                .replace_span(old.id, old.span);
            invalidated.push(old);
        }

        self.engine
            .put(&mut txn, fact_key(new_fact.id), encode_fact(&new_fact))?;
        self.engine.commit(txn)?;

        self.index
            .write()
            .expect("index poisoned")
            .insert(new_fact.id, &new_fact.span)?;
        self.provenance
            .write()
            .expect("prov poisoned")
            .insert(new_fact.id, new_fact.provenance);
        // Incrementally maintain communities: this fact bridges its endpoints.
        self.communities
            .write()
            .expect("communities poisoned")
            .add_edge(new_fact.subject, new_fact.object);

        Ok(UpsertOutcome {
            written: new_fact,
            invalidated,
        })
    }

    /// High-level ingest: intern names, build a fact valid from `valid_from`,
    /// and upsert it with the given conflict policy. Returns the new edge id.
    #[allow(clippy::too_many_arguments)]
    pub fn ingest(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        valid_from: Timestamp,
        doc: DocId,
        chunk: ChunkId,
        policy: ConflictPolicy,
    ) -> Result<EdgeId> {
        let fact = Fact {
            id: self.next_edge_id(),
            subject: self.intern_node(subject),
            predicate: self.intern_predicate(predicate),
            object: self.intern_node(object),
            span: BitemporalSpan::open(valid_from, Timestamp::MIN),
            provenance: ProvenanceRef::new(doc, chunk),
            embedding: None,
        };
        Ok(self.upsert_fact(fact, policy)?.written.id)
    }

    /// Candidate entity-merge pairs `(keep, drop, score)` discovered by lexical
    /// blocking over registered node names, with `score >= threshold`. The
    /// lower `NodeId` is the `keep` side (deterministic merge direction).
    pub fn resolution_candidates(&self, threshold: f32) -> Vec<(NodeId, NodeId, f32)> {
        let names: Vec<(NodeId, String)> = self
            .nodes
            .read()
            .expect("nodes poisoned")
            .iter()
            .map(|(id, name)| (*id, name.clone()))
            .collect();
        LexicalBlocker::new(names).candidate_pairs(threshold)
    }

    /// Resolve all candidate pairs at or above `threshold`, merging each `drop`
    /// node into its `keep` node. Returns the number of nodes merged away.
    ///
    /// Uses union-find semantics: if `a~b` and `b~c`, all three collapse into a
    /// single surviving node (the smallest id of the cluster).
    pub fn auto_resolve(&self, threshold: f32) -> Result<usize> {
        let pairs = self.resolution_candidates(threshold);
        let mut merged = 0usize;
        // Follow forwarding so transitively-merged ids resolve to the survivor.
        let mut survivor: HashMap<NodeId, NodeId> = HashMap::new();
        let resolve = |survivor: &HashMap<NodeId, NodeId>, mut n: NodeId| {
            while let Some(&s) = survivor.get(&n) {
                n = s;
            }
            n
        };
        for (keep, drop, _) in pairs {
            let keep = resolve(&survivor, keep);
            let drop = resolve(&survivor, drop);
            if keep == drop {
                continue;
            }
            let (keep, drop) = if keep <= drop {
                (keep, drop)
            } else {
                (drop, keep)
            };
            self.merge_nodes(keep, drop)?;
            survivor.insert(drop, keep);
            merged += 1;
        }
        Ok(merged)
    }

    /// Merge entity `from` into `into`: every fact referencing `from` (as
    /// subject or object) is rewritten to reference `into`, preserving each
    /// fact's bitemporal span and provenance (both keyed by edge id, so they
    /// carry over unchanged). The `from` name is repointed so future interning
    /// resolves to `into`, and the community index is updated.
    ///
    /// Note: merging may leave multiple open facts for the same
    /// subject/predicate (e.g. two sources each asserted a home city under
    /// different surface names). We dedupe exact duplicates but do not re-run
    /// contradiction resolution here; reconciling genuinely conflicting merged
    /// facts is left to a subsequent `upsert_fact`.
    pub fn merge_nodes(&self, into: NodeId, from: NodeId) -> Result<usize> {
        if into == from {
            return Ok(0);
        }
        let mut txn = self.engine.begin()?;
        let facts = self.scan_facts(&txn)?;
        let mut rewritten = 0usize;
        let mut seen: std::collections::HashSet<(NodeId, PredicateId, NodeId, i64)> =
            std::collections::HashSet::new();
        for mut fact in facts {
            let touches = fact.subject == from || fact.object == from;
            if fact.subject == from {
                fact.subject = into;
            }
            if fact.object == from {
                fact.object = into;
            }
            if !touches {
                // Still track existing edges for dedup of rewritten ones.
                seen.insert((
                    fact.subject,
                    fact.predicate,
                    fact.object,
                    fact.span.valid_from.millis(),
                ));
                continue;
            }
            let dedup_key = (
                fact.subject,
                fact.predicate,
                fact.object,
                fact.span.valid_from.millis(),
            );
            if !seen.insert(dedup_key) {
                // Exact duplicate created by the merge: drop this version.
                self.engine.delete(&mut txn, fact_key(fact.id))?;
                self.index.write().expect("index poisoned").remove(fact.id);
                continue;
            }
            self.engine
                .put(&mut txn, fact_key(fact.id), encode_fact(&fact))?;
            rewritten += 1;
        }
        self.engine.commit(txn)?;

        // Repoint the dropped name and remove its node entry.
        let from_name = self.nodes.write().expect("nodes poisoned").remove(&from);
        if let Some(name) = from_name {
            self.node_ids
                .write()
                .expect("node_ids poisoned")
                .insert(name, into);
        }
        self.communities
            .write()
            .expect("communities poisoned")
            .add_edge(into, from);
        Ok(rewritten)
    }

    /// Mark a fact as a recording error as of `at` (transaction-time
    /// invalidation / correction), as opposed to real-world supersession.
    pub fn retract_fact(&self, id: EdgeId, at: Timestamp) -> Result<()> {
        let mut txn = self.engine.begin()?;
        if let Some(bytes) = self.engine.get(&txn, &fact_key(id))? {
            let mut fact = decode_fact(&bytes)?;
            fact.span.close_tx(at);
            self.engine
                .put(&mut txn, fact_key(id), encode_fact(&fact))?;
            self.engine.commit(txn)?;
            self.index.write().expect("index poisoned").close(id, at)?;
        }
        Ok(())
    }
}
