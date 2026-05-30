//! Community detection over the entity graph.
//!
//! M4 ships **connected-component communities** (level 0) maintained
//! incrementally with a union-find: each new fact unions its subject and object
//! in near-constant time, so a new edge only ever touches the two affected
//! components instead of forcing a full rebuild (the cost advantage over batch
//! GraphRAG). Hierarchical Leiden communities and rolled-up multi-resolution
//! summaries are future work; this provides the structure and the incremental
//! maintenance contract they will build on.

use chronos_common::{NodeId, Result};
use chronos_graph_model::Community;
use std::collections::HashMap;

/// Incrementally maintained connected-component communities.
#[derive(Default)]
pub struct InMemoryCommunityIndex {
    parent: HashMap<NodeId, NodeId>,
    rank: HashMap<NodeId, u32>,
}

impl InMemoryCommunityIndex {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure(&mut self, x: NodeId) {
        self.parent.entry(x).or_insert(x);
        self.rank.entry(x).or_insert(0);
    }

    /// Find the component root of `x` without mutating (no path compression).
    fn root(&self, mut x: NodeId) -> NodeId {
        while let Some(&p) = self.parent.get(&x) {
            if p == x {
                break;
            }
            x = p;
        }
        x
    }

    /// Record an edge between two entities; unions their components. This is
    /// the incremental primitive called once per ingested fact.
    pub fn add_edge(&mut self, a: NodeId, b: NodeId) {
        self.ensure(a);
        self.ensure(b);
        let (ra, rb) = (self.root(a), self.root(b));
        if ra == rb {
            return;
        }
        let (rank_a, rank_b) = (self.rank[&ra], self.rank[&rb]);
        // Union by rank: attach the shorter tree under the taller.
        let (child, root) = if rank_a < rank_b { (ra, rb) } else { (rb, ra) };
        self.parent.insert(child, root);
        if rank_a == rank_b {
            *self.rank.get_mut(&root).unwrap() += 1;
        }
    }

    /// Register a standalone entity that may have no edges yet.
    pub fn add_node(&mut self, x: NodeId) {
        self.ensure(x);
    }

    /// All level-0 communities (one per connected component), each with a
    /// sorted member list. Summaries are left empty here and filled by callers
    /// that have access to node names (see `chronos-embedded`).
    pub fn communities(&self) -> Vec<Community> {
        let mut groups: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for &node in self.parent.keys() {
            groups.entry(self.root(node)).or_default().push(node);
        }
        let mut out: Vec<Community> = groups
            .into_iter()
            .map(|(root, mut members)| {
                members.sort();
                Community {
                    id: root.raw(),
                    members,
                    summary: None,
                    level: 0,
                }
            })
            .collect();
        out.sort_by_key(|c| c.id);
        out
    }

    /// Which community a node currently belongs to (its root id), if known.
    pub fn community_of(&self, node: NodeId) -> Option<u64> {
        if self.parent.contains_key(&node) {
            Some(self.root(node).raw())
        } else {
            None
        }
    }
}

/// Engine-facing contract for community maintenance and lookup.
pub trait CommunityIndex: Send + Sync {
    fn incremental_update(&mut self, changed: &[(NodeId, NodeId)]) -> Result<()>;
    fn communities_at_level(&self, level: u8) -> Result<Vec<Community>>;
}

impl CommunityIndex for InMemoryCommunityIndex {
    fn incremental_update(&mut self, changed: &[(NodeId, NodeId)]) -> Result<()> {
        for &(a, b) in changed {
            self.add_edge(a, b);
        }
        Ok(())
    }

    fn communities_at_level(&self, level: u8) -> Result<Vec<Community>> {
        Ok(if level == 0 {
            self.communities()
        } else {
            Vec::new()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_connected_and_separates_disjoint() {
        let mut idx = InMemoryCommunityIndex::new();
        idx.add_edge(NodeId::new(1), NodeId::new(2));
        idx.add_edge(NodeId::new(2), NodeId::new(3));
        idx.add_edge(NodeId::new(10), NodeId::new(11));

        let comms = idx.communities();
        assert_eq!(comms.len(), 2);

        // 1,2,3 share a root; 10,11 share a different root.
        assert_eq!(
            idx.community_of(NodeId::new(1)),
            idx.community_of(NodeId::new(3))
        );
        assert_ne!(
            idx.community_of(NodeId::new(1)),
            idx.community_of(NodeId::new(10))
        );

        let big = comms.iter().find(|c| c.members.len() == 3).unwrap();
        assert_eq!(
            big.members,
            vec![NodeId::new(1), NodeId::new(2), NodeId::new(3)]
        );
    }

    #[test]
    fn incremental_union_is_transitive() {
        let mut idx = InMemoryCommunityIndex::new();
        // Two components that later get bridged.
        idx.add_edge(NodeId::new(1), NodeId::new(2));
        idx.add_edge(NodeId::new(3), NodeId::new(4));
        assert_eq!(idx.communities().len(), 2);
        idx.add_edge(NodeId::new(2), NodeId::new(3));
        assert_eq!(idx.communities().len(), 1);
    }
}
