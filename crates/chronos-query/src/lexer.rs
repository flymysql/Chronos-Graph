//! Lexer. Planned backend: `logos`. Stub returns an empty token stream.

use chronos_common::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Match,
    Where,
    Return,
    AsOf,
    ValidTime,
    TransactionTime,
    Similar,
    TraverseSemantic,
    Context,
    Ident(String),
    Other(char),
}

pub fn lex(_src: &str) -> Result<Vec<Token>> {
    // TODO(M2): real tokenizer.
    Ok(Vec::new())
}
