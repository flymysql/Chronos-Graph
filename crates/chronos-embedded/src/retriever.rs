//! `MemoryRetriever`: wires the [`FactStore`](crate::FactStore) into the query
//! layer's retrieval contract and provides the end-to-end "question -> cited
//! context" pipeline.
//!
//! Since this build has no embedding model, the `SIMILAR(...)` operator is
//! scored lexically over each fact's verbalization. The pipeline is otherwise
//! the real thing: parse -> point-in-time filter -> rank -> token-budget trim
//! -> graph-to-text with citations.

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

    /// Lexical similarity in `[0, 1]`: fraction of query terms present in the
    /// fact's verbalization (case-insensitive substring match). No query terms
    /// means "match everything" (score 1.0).
    fn similarity(query_text: Option<&str>, verbalization: &str) -> f32 {
        let Some(q) = query_text else { return 1.0 };
        let hay = verbalization.to_lowercase();
        let terms: Vec<&str> = q.split_whitespace().collect();
        if terms.is_empty() {
            return 1.0;
        }
        let hits = terms
            .iter()
            .filter(|t| hay.contains(&t.to_lowercase()))
            .count();
        hits as f32 / terms.len() as f32
    }

    /// Facts visible at `at`, scored by similarity and sorted descending.
    fn ranked_facts(&self, similar_to: Option<&str>, at: AsOf) -> Result<Vec<(f32, Fact)>> {
        let mut scored: Vec<(f32, Fact)> = self
            .store
            .as_of_for(self.tenant, at)?
            .facts
            .into_iter()
            .map(|f| {
                let v = self.store.verbalize(&f);
                (Self::similarity(similar_to, &v), f)
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
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
