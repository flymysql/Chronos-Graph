//! Entity resolution.
//!
//! Detects surface variants ("OpenAI" / "Open AI" / "OpenAI Inc.") that likely
//! refer to the same entity. This crate owns **candidate detection only** —
//! dependency-free lexical blocking that stands in for an embedding-based
//! matcher (the same role `SIMILAR` plays in retrieval). The actual *merge*
//! (rewriting edges, reconciling bitemporal spans and provenance) is an engine
//! operation and lives in `chronos-embedded`'s `FactStore`, since only the
//! engine can mutate facts transactionally.

use chronos_common::{NodeId, Result};

/// Normalize a surface name for comparison: lowercase, drop punctuation,
/// collapse whitespace, and strip common organizational suffixes.
pub fn normalize(name: &str) -> String {
    let lowered = name.to_lowercase();
    let cleaned: String = lowered
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let stop = ["inc", "llc", "ltd", "corp", "co", "the"];
    cleaned
        .split_whitespace()
        .filter(|tok| !stop.contains(tok))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Token-Jaccard similarity over normalized names, in `[0.0, 1.0]`.
/// Identical normalized strings score `1.0`.
pub fn name_similarity(a: &str, b: &str) -> f32 {
    let (na, nb) = (normalize(a), normalize(b));
    if na.is_empty() || nb.is_empty() {
        return 0.0;
    }
    if na == nb {
        return 1.0;
    }
    let ta: std::collections::BTreeSet<&str> = na.split_whitespace().collect();
    let tb: std::collections::BTreeSet<&str> = nb.split_whitespace().collect();
    let inter = ta.intersection(&tb).count();
    let union = ta.union(&tb).count();
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
}

/// Lexical blocker over a set of named nodes. Produces candidate merge pairs
/// whose name similarity meets a threshold.
pub struct LexicalBlocker {
    names: Vec<(NodeId, String)>,
}

impl LexicalBlocker {
    pub fn new(names: Vec<(NodeId, String)>) -> Self {
        Self { names }
    }

    /// All unordered candidate pairs `(low_id, high_id, score)` with
    /// `score >= threshold`, sorted by descending score then id. The lower
    /// `NodeId` is listed first so merge direction is deterministic.
    pub fn candidate_pairs(&self, threshold: f32) -> Vec<(NodeId, NodeId, f32)> {
        let mut out = Vec::new();
        for i in 0..self.names.len() {
            for j in (i + 1)..self.names.len() {
                let (a_id, a_name) = &self.names[i];
                let (b_id, b_name) = &self.names[j];
                let score = name_similarity(a_name, b_name);
                if score >= threshold {
                    let (lo, hi) = if a_id <= b_id {
                        (*a_id, *b_id)
                    } else {
                        (*b_id, *a_id)
                    };
                    out.push((lo, hi, score));
                }
            }
        }
        out.sort_by(|x, y| {
            y.2.partial_cmp(&x.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(x.0.cmp(&y.0))
                .then(x.1.cmp(&y.1))
        });
        out
    }
}

/// Engine-facing contract for candidate detection. Merging is handled by the
/// engine (see `FactStore::merge_nodes`).
pub trait EntityResolver: Send + Sync {
    /// Node ids that likely refer to the same entity as `node`.
    fn candidates(&self, node: NodeId) -> Result<Vec<NodeId>>;
}

impl EntityResolver for LexicalBlocker {
    fn candidates(&self, node: NodeId) -> Result<Vec<NodeId>> {
        let target = self
            .names
            .iter()
            .find(|(id, _)| *id == node)
            .map(|(_, n)| n.clone());
        let Some(target) = target else {
            return Ok(Vec::new());
        };
        let mut hits: Vec<(NodeId, f32)> = self
            .names
            .iter()
            .filter(|(id, _)| *id != node)
            .map(|(id, name)| (*id, name_similarity(&target, name)))
            .filter(|(_, s)| *s >= 0.5)
            .collect();
        hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(hits.into_iter().map(|(id, _)| id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_suffixes_and_punctuation() {
        assert_eq!(normalize("OpenAI, Inc."), "openai");
        assert_eq!(normalize("Open AI"), "open ai");
        assert_eq!(normalize("The Acme Corp"), "acme");
    }

    #[test]
    fn similarity_detects_variants() {
        assert_eq!(name_similarity("OpenAI Inc.", "OpenAI"), 1.0);
        assert!(name_similarity("Open AI", "OpenAI") < 1.0);
        assert!(name_similarity("Alice", "Bob") < 0.5);
    }

    #[test]
    fn blocker_pairs_are_deterministic_and_ordered() {
        let names = vec![
            (NodeId::new(3), "OpenAI Inc.".to_string()),
            (NodeId::new(1), "OpenAI".to_string()),
            (NodeId::new(2), "Beijing".to_string()),
        ];
        let pairs = LexicalBlocker::new(names).candidate_pairs(0.9);
        assert_eq!(pairs.len(), 1);
        // Lower id (1) first, even though it appeared after id 3 in input.
        assert_eq!((pairs[0].0, pairs[0].1), (NodeId::new(1), NodeId::new(3)));
        assert_eq!(pairs[0].2, 1.0);
    }
}
