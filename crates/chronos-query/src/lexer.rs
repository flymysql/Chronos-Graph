//! Hand-written tokenizer for the extended openCypher subset.
//!
//! Words (keywords / identifiers) are emitted as [`Token::Word`] and matched
//! case-insensitively by the parser, which keeps the lexer tiny while still
//! supporting multi-word constructs like `AS OF VALID TIME`.

use chronos_common::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A keyword or identifier (case preserved; compared case-insensitively).
    Word(String),
    /// A double-quoted or single-quoted string literal.
    Str(String),
    /// An integer literal.
    Int(i64),
    LParen,
    RParen,
    Comma,
    Dot,
    /// `<=`
    Le,
    /// `=`
    Eq,
    Semi,
}

pub fn lex(src: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();

    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '(' => {
                out.push(Token::LParen);
                i += 1;
            }
            ')' => {
                out.push(Token::RParen);
                i += 1;
            }
            ',' => {
                out.push(Token::Comma);
                i += 1;
            }
            '.' => {
                out.push(Token::Dot);
                i += 1;
            }
            ';' => {
                out.push(Token::Semi);
                i += 1;
            }
            '=' => {
                out.push(Token::Eq);
                i += 1;
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    out.push(Token::Le);
                    i += 2;
                } else {
                    return Err(Error::Query("unexpected '<' (expected '<=')".to_string()));
                }
            }
            '"' | '\'' => {
                let quote = c;
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(Error::Query("unterminated string literal".to_string()));
                }
                out.push(Token::Str(chars[start..i].iter().collect()));
                i += 1; // closing quote
            }
            c if c.is_ascii_digit()
                || (c == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) =>
            {
                let start = i;
                if chars[i] == '-' {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n = s
                    .parse::<i64>()
                    .map_err(|_| Error::Query(format!("bad integer: {s}")))?;
                out.push(Token::Int(n));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push(Token::Word(chars[start..i].iter().collect()));
            }
            other => return Err(Error::Query(format!("unexpected character: {other:?}"))),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_example_query() {
        let toks =
            lex("MATCH (n) WHERE SIMILAR(n, \"Alice Shanghai\") RETURN CONTEXT(cite = true)")
                .unwrap();
        assert_eq!(toks[0], Token::Word("MATCH".into()));
        assert!(toks.contains(&Token::Str("Alice Shanghai".into())));
        assert!(!toks.contains(&Token::Le));
    }

    #[test]
    fn lexes_le_and_ints() {
        let toks = lex("depth <= 3, budget = 4000").unwrap();
        assert!(toks.contains(&Token::Le));
        assert!(toks.contains(&Token::Int(3)));
        assert!(toks.contains(&Token::Int(4000)));
    }
}
