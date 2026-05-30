//! `MemoryRetriever`: wires the [`FactStore`](crate::FactStore) into the query
//! layer's retrieval contract and provides the end-to-end "question -> cited
//! context" pipeline.
//!
//! Since this build has no embedding model, the `SIMILAR(...)` operator is
//! scored with a real **BM25** index over each fact's verbalization (see
//! `chronos-index`). The pipeline is otherwise the real thing: parse ->
//! point-in-time + tenant filter -> BM25 rank -> token-budget trim ->
//! graph-to-text with citations.

use crate::FactStore;
use chronos_common::{AsOf, Result, TenantId, TokenBudget};
use chronos_graph_model::{Edge, Node, Subgraph};
use chronos_provenance::Citation;
use chronos_query::executor::{ContextBlock, RetrievalOperator};
use chronos_query::CompiledQuery;
use chronos_temporal::Fact;

/// Default budget when a query omits one.
const DEFAULT_BUDGET: usize = 4096;

pub struct MemoryRetriever<'a> {
    store: &'a FactStore,
    tenant: TenantId,
}

impl<'a> MemoryRetriever<'a> {
    /// Retriever scoped to the DEFAULT tenant.
    pub fn new(store: &'a FactStore) -> Self {
        Self {
            store,
            tenant: TenantId::DEFAULT,
        }
    }

    /// Retriever scoped to `tenant`: it can only see that tenant's facts.
    pub fn new_for(store: &'a FactStore, tenant: TenantId) -> Self {
        Self { store, tenant }
    }

    /// Facts of this tenant visible at `at`, scored by BM25 relevance to the
    /// `SIMILAR(...)` text and sorted descending. With no `SIMILAR` clause,
    /// every visible fact matches (score `1.0`), ordered by recency (edge id).
    fn ranked_facts(&self, similar_to: Option<&str>, at: AsOf) -> Result<Vec<(f32, Fact)>> {
        let facts = self.store.as_of_for(self.tenant, at)?.facts;
        let mut scored: Vec<(f32, Fact)> = match similar_to {
            None => facts.into_iter().map(|f| (1.0, f)).collect(),
            Some(q) => {
                let scores = self.store.bm25_scores(q);
                facts
                    .into_iter()
                    .filter_map(|f| scores.get(&f.id).copied().map(|s| (s, f)))
                    .filter(|(s, _)| *s > 0.0)
                    .collect()
            }
        };
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.1.id.raw().cmp(&a.1.id.raw()))
        });
        Ok(scored)
    }

    /// End-to-end: compile `src`, retrieve, budget-trim, and serialize to a
    /// cited [`ContextBlock`].
    pub fn answer(&self, src: &str) -> Result<ContextBlock> {
        let compiled = chronos_query::compile(src)?;
        let at = compiled.query.as_of.unwrap_or_else(AsOf::now);
        let budget = compiled
            .query
            .budget
            .unwrap_or(TokenBudget(DEFAULT_BUDGET))
            .0;
        let cite = compiled.query.cite;

        let ranked = self.ranked_facts(compiled.query.similar_to.as_deref(), at)?;

        let mut used = 0usize;
        let mut lines = Vec::new();
        let mut citations = Vec::new();
        for (_score, fact) in ranked {
            let text = self.store.verbalize(&fact);
            let cost = text.split_whitespace().count().max(1);
            if used + cost > budget {
                break;
            }
            used += cost;
            lines.push(format!("- {text}"));
            if cite {
                if let Some(src) = self.store.provenance_of(fact.id) {
                    citations.push(Citation {
                        source: src,
                        snippet: Some(text),
                    });
                }
            }
        }

        Ok(ContextBlock {
            text: lines.join("\n"),
            citations,
        })
    }
}

impl RetrievalOperator for MemoryRetriever<'_> {
    fn retrieve(&self, query: &CompiledQuery, budget: TokenBudget, at: AsOf) -> Result<Subgraph> {
        let ranked = self.ranked_facts(query.query.similar_to.as_deref(), at)?;

        let mut sg = Subgraph::default();
        let mut used = 0usize;
        let mut seen_nodes = std::collections::BTreeSet::new();
        for (_score, fact) in ranked {
            let cost = self
                .store
                .verbalize(&fact)
                .split_whitespace()
                .count()
                .max(1);
            if used + cost > budget.0 {
                break;
            }
            used += cost;
            for nid in [fact.subject, fact.object] {
                if seen_nodes.insert(nid.raw()) {
                    sg.nodes.push(Node::new(nid, self.store.node_name(nid)));
                }
            }
            sg.edges.push(Edge::new(
                fact.id,
                fact.subject,
                fact.predicate,
                fact.object,
            ));
        }
        Ok(sg)
    }
}
