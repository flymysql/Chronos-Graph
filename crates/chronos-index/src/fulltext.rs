//! Full-text / BM25 index.
//!
//! A real in-memory Okapi BM25 implementation (a `tantivy` backend is planned
//! for the persistent build). The core [`Bm25Index<K>`] is generic over the key
//! type so it can rank source documents (`DocId`) *or* facts (`EdgeId`) — the
//! engine indexes each fact's verbalized text to power the `SIMILAR(...)`
//! operator without an embedding model.

use chronos_common::{DocId, Result};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, Copy)]
pub struct Bm25Hit {
    pub id: DocId,
    pub score: f32,
}

/// Okapi BM25 free parameters (standard defaults).
const K1: f32 = 1.5;
const B: f32 = 0.75;

/// Tokenize: lowercase, split on non-alphanumeric, drop empties.
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// Generic in-memory BM25 index keyed by `K`.
pub struct Bm25Index<K: Eq + Hash + Clone> {
    /// Per-document token lists (kept for re-indexing / length).
    docs: HashMap<K, Vec<String>>,
    /// term -> document frequency (number of docs containing the term).
    df: HashMap<String, usize>,
    total_len: usize,
}

impl<K: Eq + Hash + Clone> Default for Bm25Index<K> {
    fn default() -> Self {
        Self {
            docs: HashMap::new(),
            df: HashMap::new(),
            total_len: 0,
        }
    }
}

impl<K: Eq + Hash + Clone> Bm25Index<K> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    fn unindex(&mut self, key: &K) {
        if let Some(tokens) = self.docs.remove(key) {
            self.total_len -= tokens.len();
            let unique: std::collections::BTreeSet<&String> = tokens.iter().collect();
            for term in unique {
                if let Some(c) = self.df.get_mut(term) {
                    *c -= 1;
                    if *c == 0 {
                        self.df.remove(term);
                    }
                }
            }
        }
    }

    /// Index (or re-index) `text` under `key`, overwriting any prior entry.
    pub fn add(&mut self, key: K, text: &str) {
        self.unindex(&key);
        let tokens = tokenize(text);
        self.total_len += tokens.len();
        let unique: std::collections::BTreeSet<&String> = tokens.iter().collect();
        for term in unique {
            *self.df.entry(term.clone()).or_insert(0) += 1;
        }
        self.docs.insert(key, tokens);
    }

    /// Remove `key` from the index.
    pub fn remove(&mut self, key: &K) {
        self.unindex(key);
    }

    fn avgdl(&self) -> f32 {
        if self.docs.is_empty() {
            0.0
        } else {
            self.total_len as f32 / self.docs.len() as f32
        }
    }

    /// BM25 score for a single document's token list against query terms.
    fn score_doc(&self, query_terms: &[String], tokens: &[String], avgdl: f32, n: f32) -> f32 {
        let dl = tokens.len() as f32;
        let mut tf: HashMap<&str, f32> = HashMap::new();
        for t in tokens {
            *tf.entry(t.as_str()).or_insert(0.0) += 1.0;
        }
        let mut score = 0.0;
        for q in query_terms {
            let f = match tf.get(q.as_str()) {
                Some(f) => *f,
                None => continue,
            };
            let df = *self.df.get(q).unwrap_or(&0) as f32;
            // BM25+ style idf, always non-negative.
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            let denom = f + K1 * (1.0 - B + B * dl / avgdl);
            score += idf * (f * (K1 + 1.0)) / denom;
        }
        score
    }

    /// Top-`k` keys by BM25 score for `query` (descending), score `> 0` only.
    pub fn search(&self, query: &str, k: usize) -> Vec<(K, f32)> {
        let query_terms = tokenize(query);
        if query_terms.is_empty() || self.docs.is_empty() {
            return Vec::new();
        }
        let avgdl = self.avgdl();
        let n = self.docs.len() as f32;
        let mut scored: Vec<(K, f32)> = self
            .docs
            .iter()
            .map(|(key, tokens)| (key.clone(), self.score_doc(&query_terms, tokens, avgdl, n)))
            .filter(|(_, s)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }
}

pub trait FullTextIndex: Send + Sync {
    fn index(&mut self, id: DocId, text: &str) -> Result<()>;
    fn search_bm25(&self, query: &str, k: usize) -> Result<Vec<Bm25Hit>>;
}

impl FullTextIndex for Bm25Index<DocId> {
    fn index(&mut self, id: DocId, text: &str) -> Result<()> {
        self.add(id, text);
        Ok(())
    }

    fn search_bm25(&self, query: &str, k: usize) -> Result<Vec<Bm25Hit>> {
        Ok(self
            .search(query, k)
            .into_iter()
            .map(|(id, score)| Bm25Hit { id, score })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_more_relevant_doc_higher() {
        let mut idx: Bm25Index<DocId> = Bm25Index::new();
        idx.add(DocId::new(1), "the quick brown fox");
        idx.add(DocId::new(2), "the lazy dog sleeps");
        idx.add(DocId::new(3), "quick quick fox fox jumps");

        let hits = idx.search("quick fox", 10);
        assert_eq!(hits[0].0, DocId::new(3), "doc 3 has the most matches");
        assert!(hits.iter().all(|(_, s)| *s > 0.0));
        // Doc 2 shares no query terms and must not appear.
        assert!(!hits.iter().any(|(id, _)| *id == DocId::new(2)));
    }

    #[test]
    fn reindex_overwrites_previous_text() {
        let mut idx: Bm25Index<DocId> = Bm25Index::new();
        idx.add(DocId::new(1), "apple banana");
        idx.add(DocId::new(1), "cherry");
        assert!(idx.search("apple", 10).is_empty());
        assert_eq!(idx.search("cherry", 10).len(), 1);
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn empty_query_returns_nothing() {
        let mut idx: Bm25Index<DocId> = Bm25Index::new();
        idx.add(DocId::new(1), "hello world");
        assert!(idx.search("", 10).is_empty());
    }
}
