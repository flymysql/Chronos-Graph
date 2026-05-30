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
use chronos_common::{AsOf, EdgeId, ProvenanceRef, Result, Timestamp};
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
    next_edge: AtomicU64,
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
            next_edge: AtomicU64::new(1),
        }
    }

    /// Allocate a fresh edge id.
    pub fn next_edge_id(&self) -> EdgeId {
        EdgeId::new(self.next_edge.fetch_add(1, Ordering::SeqCst))
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

        Ok(UpsertOutcome {
            written: new_fact,
            invalidated,
        })
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
