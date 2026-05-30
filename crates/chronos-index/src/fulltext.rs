//! Full-text / BM25 index abstraction (tantivy backend planned).

use chronos_common::{DocId, Result};

#[derive(Debug, Clone, Copy)]
pub struct Bm25Hit {
    pub id: DocId,
    pub score: f32,
}

pub trait FullTextIndex: Send + Sync {
    fn index(&mut self, id: DocId, text: &str) -> Result<()>;
    fn search_bm25(&self, query: &str, k: usize) -> Result<Vec<Bm25Hit>>;
}
