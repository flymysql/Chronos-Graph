//! Fixed-layout byte (de)serialization for `Fact` records.
//!
//! Kept dependency-free (no serde) for the skeleton. Layout is little-endian:
//! ids (4 x u64) | span (4 timestamps, each an optional i64) | provenance
//! (2 x u64) | embedding (optional u64).

use chronos_common::{
    BitemporalSpan, ChunkId, DocId, EdgeId, Error, NodeId, PredicateId, ProvenanceRef, Result,
    Timestamp, VectorId,
};
use chronos_temporal::Fact;

/// Storage key for a fact: a single record per edge id (overwritten in place
/// when a span is closed), so facts can be fetched directly by id.
pub fn fact_key(id: EdgeId) -> Vec<u8> {
    let mut k = Vec::with_capacity(1 + 8);
    k.push(b'F');
    k.extend_from_slice(&id.raw().to_le_bytes());
    k
}

/// Key-space prefix covering all fact records.
pub const FACT_PREFIX: &[u8] = b"F";
/// Key-space prefix for node-name records (`N` + node id -> UTF-8 name).
pub const NODE_PREFIX: &[u8] = b"N";
/// Key-space prefix for predicate-name records (`P` + predicate id -> name).
pub const PRED_PREFIX: &[u8] = b"P";
/// Key-space prefix for per-edge tenant assignments (`T` + edge id -> u64 LE).
pub const TENANT_PREFIX: &[u8] = b"T";

fn id_key(prefix: &[u8], id: u64) -> Vec<u8> {
    let mut k = Vec::with_capacity(prefix.len() + 8);
    k.extend_from_slice(prefix);
    k.extend_from_slice(&id.to_le_bytes());
    k
}

/// Storage key for a node's name.
pub fn node_key(id: NodeId) -> Vec<u8> {
    id_key(NODE_PREFIX, id.raw())
}

/// Storage key for a predicate's name.
pub fn pred_key(id: PredicateId) -> Vec<u8> {
    id_key(PRED_PREFIX, id.raw())
}

/// Storage key for an edge's tenant assignment.
pub fn tenant_key(id: EdgeId) -> Vec<u8> {
    id_key(TENANT_PREFIX, id.raw())
}

/// Parse the trailing u64 id from a prefixed key.
pub fn id_from_key(prefix: &[u8], key: &[u8]) -> Option<u64> {
    if key.len() == prefix.len() + 8 && key.starts_with(prefix) {
        let bytes: [u8; 8] = key[prefix.len()..].try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
    } else {
        None
    }
}

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }
    fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn opt_ts(&mut self, v: Option<Timestamp>) {
        match v {
            Some(t) => {
                self.buf.push(1);
                self.i64(t.millis());
            }
            None => {
                self.buf.push(0);
                self.i64(0);
            }
        }
    }
    fn opt_u64(&mut self, v: Option<u64>) {
        match v {
            Some(x) => {
                self.buf.push(1);
                self.u64(x);
            }
            None => {
                self.buf.push(0);
                self.u64(0);
            }
        }
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.pos + n > self.buf.len() {
            return Err(Error::Storage("fact record truncated".to_string()));
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u64(&mut self) -> Result<u64> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes(b.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64> {
        let b = self.take(8)?;
        Ok(i64::from_le_bytes(b.try_into().unwrap()))
    }
    fn opt_ts(&mut self) -> Result<Option<Timestamp>> {
        let flag = self.take(1)?[0];
        let raw = self.i64()?;
        Ok(if flag == 1 {
            Some(Timestamp::from_millis(raw))
        } else {
            None
        })
    }
    fn opt_u64(&mut self) -> Result<Option<u64>> {
        let flag = self.take(1)?[0];
        let raw = self.u64()?;
        Ok(if flag == 1 { Some(raw) } else { None })
    }
}

pub fn encode_fact(fact: &Fact) -> Vec<u8> {
    let mut w = Writer::new();
    w.u64(fact.id.raw());
    w.u64(fact.subject.raw());
    w.u64(fact.predicate.raw());
    w.u64(fact.object.raw());
    w.i64(fact.span.valid_from.millis());
    w.opt_ts(fact.span.valid_to);
    w.i64(fact.span.tx_from.millis());
    w.opt_ts(fact.span.tx_to);
    w.u64(fact.provenance.doc.raw());
    w.u64(fact.provenance.chunk.raw());
    w.opt_u64(fact.embedding.map(|v| v.raw()));
    w.buf
}

pub fn decode_fact(bytes: &[u8]) -> Result<Fact> {
    let mut r = Reader::new(bytes);
    let id = EdgeId::new(r.u64()?);
    let subject = NodeId::new(r.u64()?);
    let predicate = PredicateId::new(r.u64()?);
    let object = NodeId::new(r.u64()?);
    let valid_from = Timestamp::from_millis(r.i64()?);
    let valid_to = r.opt_ts()?;
    let tx_from = Timestamp::from_millis(r.i64()?);
    let tx_to = r.opt_ts()?;
    let doc = DocId::new(r.u64()?);
    let chunk = ChunkId::new(r.u64()?);
    let embedding = r.opt_u64()?.map(VectorId::new);
    Ok(Fact {
        id,
        subject,
        predicate,
        object,
        span: BitemporalSpan {
            valid_from,
            valid_to,
            tx_from,
            tx_to,
        },
        provenance: ProvenanceRef::new(doc, chunk),
        embedding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let fact = Fact {
            id: EdgeId::new(42),
            subject: NodeId::new(1),
            predicate: PredicateId::new(2),
            object: NodeId::new(3),
            span: BitemporalSpan {
                valid_from: Timestamp::from_millis(100),
                valid_to: Some(Timestamp::from_millis(200)),
                tx_from: Timestamp::from_millis(50),
                tx_to: None,
            },
            provenance: ProvenanceRef::new(DocId::new(7), ChunkId::new(9)),
            embedding: Some(VectorId::new(5)),
        };
        let decoded = decode_fact(&encode_fact(&fact)).unwrap();
        assert_eq!(decoded.id, fact.id);
        assert_eq!(decoded.span, fact.span);
        assert_eq!(decoded.provenance, fact.provenance);
        assert_eq!(decoded.embedding, fact.embedding);
    }

    #[test]
    fn truncated_record_errors() {
        assert!(decode_fact(&[0u8; 4]).is_err());
    }
}
