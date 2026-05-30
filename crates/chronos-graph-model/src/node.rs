//! Nodes (entities) and their properties.

use chronos_common::{NodeId, VectorId};
use std::collections::HashMap;

/// A property value attached to a node or edge.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

/// An entity node. Entities are persistent; temporal validity lives on edges.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    /// Entity type label (e.g. "Person", "Org"). Open-world if absent.
    pub label: Option<String>,
    /// Canonical name after entity resolution.
    pub name: String,
    pub properties: HashMap<String, PropertyValue>,
    /// Embedding of the node's textual summary, if indexed.
    pub embedding: Option<VectorId>,
}

impl Node {
    pub fn new(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            label: None,
            name: name.into(),
            properties: HashMap::new(),
            embedding: None,
        }
    }
}
