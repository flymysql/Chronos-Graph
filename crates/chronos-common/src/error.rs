//! Crate-wide error type. Kept dependency-free in the skeleton; can later be
//! swapped for `thiserror` without changing the public surface.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// A requested item was not found.
    NotFound(String),
    /// The storage layer reported a failure.
    Storage(String),
    /// A query failed to parse or compile.
    Query(String),
    /// A bitemporal invariant was violated (e.g. overlapping open spans).
    Temporal(String),
    /// Catch-all for not-yet-implemented engine paths.
    Unimplemented(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(m) => write!(f, "not found: {m}"),
            Error::Storage(m) => write!(f, "storage error: {m}"),
            Error::Query(m) => write!(f, "query error: {m}"),
            Error::Temporal(m) => write!(f, "temporal error: {m}"),
            Error::Unimplemented(m) => write!(f, "unimplemented: {m}"),
        }
    }
}

impl std::error::Error for Error {}
