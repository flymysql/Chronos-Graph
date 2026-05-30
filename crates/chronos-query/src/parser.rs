//! Parser: tokens -> AST. Planned backend: hand-written recursive descent over
//! an openCypher subset, extended with temporal/semantic clauses.

use crate::ast::Query;
use crate::lexer::Token;
use chronos_common::Result;

pub fn parse(_tokens: Vec<Token>) -> Result<Query> {
    // TODO(M2): real grammar.
    Ok(Query::default())
}
