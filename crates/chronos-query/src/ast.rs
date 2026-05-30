//! Abstract syntax tree for the query language.

use chronos_common::{AsOf, TokenBudget};

/// A parsed query. The grammar is intentionally forgiving: every clause beyond
/// `RETURN` is optional, so partial queries still compile.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Query {
    /// Pattern variable bound by `MATCH (var ...)`, if present.
    pub match_var: Option<String>,
    /// Optional point-in-time selector from `AS OF [VALID|TRANSACTION] TIME n`.
    pub as_of: Option<AsOf>,
    /// Optional semantic similarity target text from `SIMILAR(var, "text")`.
    pub similar_to: Option<String>,
    /// Max traversal depth from `TRAVERSE SEMANTIC(depth <= n)`.
    pub max_depth: Option<u32>,
    /// Token budget from `TRAVERSE SEMANTIC(... budget = n tokens)`.
    pub budget: Option<TokenBudget>,
    /// Whether `RETURN CONTEXT(cite = true)` was requested.
    pub return_context: bool,
    /// Whether the requested context should carry citations.
    pub cite: bool,
    /// Plain `RETURN <ident>, ...` projection (when not returning CONTEXT).
    pub return_idents: Vec<String>,
}
