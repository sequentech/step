// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Quorum safety (Vitrine's `specs/policy.md`, section 3, rule 5): the split
//! cost of a quorum, the least number of honest witnesses that would have to
//! become faulty for two inconsistent checkpoints to both meet it, and its
//! party split cost, the same count in parties.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Witness { party: String },
    Group { k: usize, members: Vec<String> },
}

/// Witnesses and groups by name, and the quorum's name. Groups name only
/// nodes defined before them, and each node is a member of at most one
/// group, so the nodes form a tree.
#[derive(Debug, Clone, Default)]
pub struct QuorumTree {
    pub nodes: BTreeMap<String, Node>,
    pub root: String,
}

impl QuorumTree {
    fn cost(&self, name: &str, faulty: &BTreeSet<String>) -> usize {
        match self.nodes.get(name) {
            None => 0,
            Some(Node::Witness { party }) => usize::from(!faulty.contains(party)),
            Some(Node::Group { k, members }) => {
                let n = members.len();
                if 2 * k <= n {
                    return 0;
                }
                let mut costs: Vec<usize> = members.iter().map(|m| self.cost(m, faulty)).collect();
                costs.sort_unstable();
                costs.iter().take(2 * k - n).sum()
            }
        }
    }

    fn parties(&self, name: &str, out: &mut BTreeSet<String>) {
        match self.nodes.get(name) {
            None => {}
            Some(Node::Witness { party }) => {
                out.insert(party.clone());
            }
            Some(Node::Group { members, .. }) => {
                for m in members {
                    self.parties(m, out);
                }
            }
        }
    }
}

/// The split cost with the witnesses of `faulty` parties counted as faulty.
pub fn split_cost(tree: &QuorumTree, faulty: &BTreeSet<String>) -> usize {
    tree.cost(&tree.root, faulty)
}

/// The least number of parties, besides `faulty`, whose witnesses would all
/// have to become faulty for the split cost to be 0, found by brute force
/// over sets of parties, smallest first.
pub fn party_split_cost(tree: &QuorumTree, faulty: &BTreeSet<String>) -> usize {
    if split_cost(tree, faulty) == 0 {
        return 0;
    }
    let mut parties = BTreeSet::new();
    tree.parties(&tree.root, &mut parties);
    let candidates: Vec<String> = parties.difference(faulty).cloned().collect();
    let mut assumed = faulty.clone();
    for size in 1..=candidates.len() {
        if splits(tree, &mut assumed, &candidates, 0, size) {
            return size;
        }
    }
    candidates.len()
}

fn splits(
    tree: &QuorumTree,
    faulty: &mut BTreeSet<String>,
    candidates: &[String],
    start: usize,
    left: usize,
) -> bool {
    if left == 0 {
        return split_cost(tree, faulty) == 0;
    }
    for i in start..candidates.len() {
        if candidates.len() - i < left {
            break;
        }
        faulty.insert(candidates[i].clone());
        let found = splits(tree, faulty, candidates, i + 1, left - 1);
        faulty.remove(&candidates[i]);
        if found {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn witness(party: &str) -> Node {
        Node::Witness {
            party: party.into(),
        }
    }

    fn group(k: usize, members: &[&str]) -> Node {
        Node::Group {
            k,
            members: members.iter().map(|m| m.to_string()).collect(),
        }
    }

    fn tree(nodes: Vec<(&str, Node)>, root: &str) -> QuorumTree {
        QuorumTree {
            nodes: nodes
                .into_iter()
                .map(|(n, node)| (n.to_string(), node))
                .collect(),
            root: root.into(),
        }
    }

    fn parties(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn a_single_witness_costs_one_unless_faulty() {
        let t = tree(vec![("w", witness("a"))], "w");
        assert_eq!(split_cost(&t, &parties(&[])), 1);
        assert_eq!(party_split_cost(&t, &parties(&[])), 1);
        assert_eq!(split_cost(&t, &parties(&["a"])), 0);
        assert_eq!(party_split_cost(&t, &parties(&["a"])), 0);
    }

    #[test]
    fn a_group_costs_its_overlap() {
        let members = vec![
            ("a", witness("a")),
            ("b", witness("b")),
            ("c", witness("c")),
            ("d", witness("d")),
        ];
        let with = |k| {
            let mut nodes = members.clone();
            nodes.push(("g", group(k, &["a", "b", "c", "d"])));
            tree(nodes, "g")
        };
        assert_eq!(split_cost(&with(2), &parties(&[])), 0);
        assert_eq!(split_cost(&with(3), &parties(&[])), 2);
        assert_eq!(split_cost(&with(4), &parties(&[])), 4);
        assert_eq!(split_cost(&with(3), &parties(&["a"])), 1);
        assert_eq!(party_split_cost(&with(3), &parties(&["a"])), 1);
        assert_eq!(party_split_cost(&with(4), &parties(&[])), 4);
    }

    #[test]
    fn witnesses_of_one_party_fail_together() {
        let t = tree(
            vec![
                ("a", witness("p")),
                ("b", witness("p")),
                ("c", witness("q")),
                ("g", group(3, &["a", "b", "c"])),
            ],
            "g",
        );
        assert_eq!(split_cost(&t, &parties(&[])), 3);
        assert_eq!(party_split_cost(&t, &parties(&[])), 2);
        assert_eq!(party_split_cost(&t, &parties(&["q"])), 1);
    }

    #[test]
    fn nested_groups_add_their_members_costs() {
        let t = tree(
            vec![
                ("a", witness("a")),
                ("b", witness("b")),
                ("c", witness("c")),
                ("d", witness("d")),
                ("e", witness("e")),
                ("f", witness("f")),
                ("g1", group(2, &["a", "b", "c"])),
                ("g2", group(2, &["d", "e", "f"])),
                ("q", group(2, &["g1", "g2"])),
            ],
            "q",
        );
        assert_eq!(split_cost(&t, &parties(&[])), 2);
        assert_eq!(party_split_cost(&t, &parties(&[])), 2);
        assert_eq!(party_split_cost(&t, &parties(&["a"])), 1);
    }
}
