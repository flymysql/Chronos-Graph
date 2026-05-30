//! Recursive-descent parser for the extended openCypher subset.
//!
//! Supported shape (every clause after RETURN is optional, order as below):
//!
//! ```text
//! MATCH ( var ... )
//! WHERE SIMILAR( var , "text" ) [AND ...]
//! AS OF (VALID | TRANSACTION) TIME <int>
//! TRAVERSE SEMANTIC( depth <= <int> , budget = <int> tokens )
//! RETURN ( CONTEXT( cite = true|false ) | ident [, ident ...] )
//! ```

use crate::ast::Query;
use crate::lexer::Token;
use chronos_common::{AsOf, Error, Result, Timestamp, TokenBudget};

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// If the next token is the given keyword (case-insensitive word), consume it.
    fn eat_kw(&mut self, kw: &str) -> bool {
        if let Some(Token::Word(w)) = self.peek() {
            if w.eq_ignore_ascii_case(kw) {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn expect(&mut self, t: Token) -> Result<()> {
        match self.next() {
            Some(ref got) if *got == t => Ok(()),
            other => Err(Error::Query(format!("expected {t:?}, found {other:?}"))),
        }
    }

    fn expect_int(&mut self) -> Result<i64> {
        match self.next() {
            Some(Token::Int(n)) => Ok(n),
            other => Err(Error::Query(format!("expected integer, found {other:?}"))),
        }
    }

    fn expect_word(&mut self) -> Result<String> {
        match self.next() {
            Some(Token::Word(w)) => Ok(w),
            other => Err(Error::Query(format!(
                "expected identifier, found {other:?}"
            ))),
        }
    }

    fn expect_str(&mut self) -> Result<String> {
        match self.next() {
            Some(Token::Str(s)) => Ok(s),
            other => Err(Error::Query(format!("expected string, found {other:?}"))),
        }
    }

    /// Consume tokens until the matching `)` for an already-consumed `(`.
    fn skip_parens(&mut self) -> Result<()> {
        let mut depth = 1;
        while depth > 0 {
            match self.next() {
                Some(Token::LParen) => depth += 1,
                Some(Token::RParen) => depth -= 1,
                Some(_) => {}
                None => return Err(Error::Query("unbalanced parentheses".to_string())),
            }
        }
        Ok(())
    }

    fn parse(&mut self) -> Result<Query> {
        let mut q = Query::default();

        // MATCH ( var ... )
        if self.eat_kw("MATCH") {
            self.expect(Token::LParen)?;
            if let Some(Token::Word(_)) = self.peek() {
                q.match_var = Some(self.expect_word()?);
            }
            self.skip_parens()?;
        }

        // WHERE ... (we only extract SIMILAR; other predicates are skipped)
        if self.eat_kw("WHERE") {
            self.parse_where(&mut q)?;
        }

        // AS OF [VALID|TRANSACTION] TIME <int>
        if self.eat_kw("AS") {
            if !self.eat_kw("OF") {
                return Err(Error::Query("expected OF after AS".to_string()));
            }
            self.parse_as_of(&mut q)?;
        }

        // TRAVERSE SEMANTIC( ... )
        if self.eat_kw("TRAVERSE") {
            if !self.eat_kw("SEMANTIC") {
                return Err(Error::Query("expected SEMANTIC after TRAVERSE".to_string()));
            }
            self.parse_traverse(&mut q)?;
        }

        // RETURN ...
        if self.eat_kw("RETURN") {
            self.parse_return(&mut q)?;
        }

        let _ = self.eat_semi();
        Ok(q)
    }

    fn eat_semi(&mut self) -> bool {
        if matches!(self.peek(), Some(Token::Semi)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_where(&mut self, q: &mut Query) -> Result<()> {
        loop {
            if self.eat_kw("SIMILAR") {
                self.expect(Token::LParen)?;
                // SIMILAR(var, "text") — variable is optional/ignored.
                if let Some(Token::Word(_)) = self.peek() {
                    let _ = self.expect_word();
                    let _ = self.eat_comma();
                }
                q.similar_to = Some(self.expect_str()?);
                self.expect(Token::RParen)?;
            } else {
                // Unknown predicate: consume one token to make progress.
                match self.peek() {
                    Some(Token::Word(w))
                        if w.eq_ignore_ascii_case("AS")
                            || w.eq_ignore_ascii_case("TRAVERSE")
                            || w.eq_ignore_ascii_case("RETURN") =>
                    {
                        break
                    }
                    Some(_) => {
                        self.pos += 1;
                        continue;
                    }
                    None => break,
                }
            }
            if !self.eat_kw("AND") {
                break;
            }
        }
        Ok(())
    }

    fn eat_comma(&mut self) -> bool {
        if matches!(self.peek(), Some(Token::Comma)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_as_of(&mut self, q: &mut Query) -> Result<()> {
        let valid = if self.eat_kw("VALID") {
            true
        } else if self.eat_kw("TRANSACTION") {
            false
        } else {
            return Err(Error::Query("expected VALID or TRANSACTION".to_string()));
        };
        if !self.eat_kw("TIME") {
            return Err(Error::Query("expected TIME".to_string()));
        }
        let n = self.expect_int()?;
        let ts = Timestamp::from_millis(n);
        q.as_of = Some(if valid {
            AsOf::new(ts, Timestamp::MAX)
        } else {
            AsOf::new(Timestamp::MAX, ts)
        });
        Ok(())
    }

    fn parse_traverse(&mut self, q: &mut Query) -> Result<()> {
        self.expect(Token::LParen)?;
        loop {
            if self.eat_kw("depth") {
                self.expect(Token::Le)?;
                q.max_depth = Some(self.expect_int()? as u32);
            } else if self.eat_kw("budget") {
                self.expect(Token::Eq)?;
                let n = self.expect_int()?;
                q.budget = Some(TokenBudget::new(n.max(0) as usize));
                let _ = self.eat_kw("tokens");
            } else if self.eat_comma() {
                continue;
            } else {
                break;
            }
        }
        self.expect(Token::RParen)?;
        Ok(())
    }

    fn parse_return(&mut self, q: &mut Query) -> Result<()> {
        if self.eat_kw("CONTEXT") {
            q.return_context = true;
            self.expect(Token::LParen)?;
            if self.eat_kw("cite") {
                self.expect(Token::Eq)?;
                q.cite = self.parse_bool()?;
            }
            self.expect(Token::RParen)?;
        } else {
            loop {
                q.return_idents.push(self.expect_word()?);
                if !self.eat_comma() {
                    break;
                }
            }
        }
        Ok(())
    }

    fn parse_bool(&mut self) -> Result<bool> {
        if self.eat_kw("true") {
            Ok(true)
        } else if self.eat_kw("false") {
            Ok(false)
        } else {
            Err(Error::Query("expected true or false".to_string()))
        }
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Query> {
    let mut p = Parser {
        toks: tokens,
        pos: 0,
    };
    p.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_str(s: &str) -> Query {
        parse(lex(s).unwrap()).unwrap()
    }

    #[test]
    fn parses_full_example() {
        let q = parse_str(
            "MATCH (n) WHERE SIMILAR(n, \"Alice Shanghai\") \
             AS OF VALID TIME 1500 \
             TRAVERSE SEMANTIC(depth <= 3, budget = 4000 tokens) \
             RETURN CONTEXT(cite = true)",
        );
        assert_eq!(q.match_var.as_deref(), Some("n"));
        assert_eq!(q.similar_to.as_deref(), Some("Alice Shanghai"));
        assert_eq!(q.max_depth, Some(3));
        assert_eq!(q.budget, Some(TokenBudget::new(4000)));
        assert!(q.return_context && q.cite);
        let at = q.as_of.unwrap();
        assert_eq!(at.valid_time, Timestamp::from_millis(1500));
    }

    #[test]
    fn parses_minimal_return_idents() {
        let q = parse_str("MATCH (n) RETURN n");
        assert_eq!(q.return_idents, vec!["n".to_string()]);
        assert!(!q.return_context);
    }

    #[test]
    fn parses_transaction_time() {
        let q = parse_str("AS OF TRANSACTION TIME 42 RETURN CONTEXT(cite = false)");
        let at = q.as_of.unwrap();
        assert_eq!(at.tx_time, Timestamp::from_millis(42));
        assert!(!q.cite);
    }
}
