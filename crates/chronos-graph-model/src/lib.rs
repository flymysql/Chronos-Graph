//! Graph model: property-graph primitives plus the three hierarchical tiers
//! used for agent memory (episode / entity / community), following the Zep
//! temporal knowledge-graph design.

pub mod edge;
pub mod node;
pub mod subgraph;

pub use edge::Edge;
pub use node::{Node, PropertyValue};
pub use subgraph::{Community, Episode, Subgraph, Tier};
