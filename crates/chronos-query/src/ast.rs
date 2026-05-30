//! Abstract syntax tree for the query language.

use chronos_common::{AsOf, TokenBudget};

/// A parsed query. Only the fields needed to thread the skeleton through are
/// modelled; pattern/clause detail is filled in at M2.
#[derive(Debug, Default, Clone)]
pub struct Query {
    /// Optional point-in-time selector from `AS OF ... TIME`.
    pub as_of: Option<AsOf>,
    /// Optional semantic similarity target text from `SIMILAR(...)`.
    pub similar_to: Option<String>,
    /// Max traversal depth from `TRAVERSE SEMANTIC(depth <= n)`.
    pub max_depth: Option<u32>,
    /// Token budget from `... budget = n tokens`.
    pub budget: Option<TokenBudget>,
    /// Whether `RETURN CONTEXT(cite = true)` was requested.
    pub return_context_cited: bool,
}
