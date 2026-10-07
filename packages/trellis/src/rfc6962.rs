// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! RFC 6962 Merkle tree arithmetic over persisted perfect subtrees.
//!
//! A tree of `n` leaves is stored as the hashes of its *perfect subtrees*: aligned
//! blocks of `2^level` leaves. Once complete, a perfect subtree never changes, so its
//! hash is written once. The right edge of the tree (the [`Frontier`]) holds the roots
//! of the perfect subtrees that make up `n`, from which the tree root is folded.
//! Inclusion and consistency proofs for any historical size only need the hashes of
//! `O(log n)` perfect subtrees. Hashing follows RFC 6962 (and `ct-merkle`):
//! leaves are `SHA-256(0x00 || data)` and parents `SHA-256(0x01 || left || right)`.
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::hash::BuildHasher;

/// A SHA-256 digest.
pub type Hash = [u8; 32];

/// Root of the empty tree, `SHA-256("")`.
#[must_use]
pub fn empty_root() -> Hash {
    Sha256::digest(b"").into()
}

/// RFC 6962 leaf hash of the stored leaf data.
#[must_use]
pub fn leaf_hash(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([0x00]);
    hasher.update(data);
    hasher.finalize().into()
}

/// RFC 6962 parent hash.
#[must_use]
pub fn node_hash(left: &Hash, right: &Hash) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([0x01]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// A perfect subtree covering leaves `[index << level, (index + 1) << level)`.
///
/// Level 0 nodes are leaves; their hash is the leaf hash of the stored leaf data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Node {
    /// Height of the subtree; it covers `2^level` leaves.
    pub level: u8,
    /// Position among the subtrees of the same level.
    pub index: u64,
}

/// Errors from tree arithmetic.
#[derive(Debug, PartialEq, Eq)]
pub enum TreeError {
    /// A right edge does not match its tree size.
    MalformedFrontier,
    /// A needed perfect subtree hash is not available.
    MissingNode(Node),
    /// The requested sizes or positions are outside the tree.
    OutOfRange,
}

impl std::fmt::Display for TreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedFrontier => {
                write!(f, "Merkle right edge does not match the tree size")
            }
            Self::MissingNode(node) => write!(
                f,
                "Merkle node at level {} index {} is missing",
                node.level, node.index
            ),
            Self::OutOfRange => write!(f, "Merkle position or size is out of range"),
        }
    }
}

impl std::error::Error for TreeError {}

/// Read access to perfect subtree hashes.
pub trait Nodes {
    /// Hash of a perfect subtree; for level 0, the leaf hash.
    fn get(&self, node: Node) -> Option<Hash>;
}

impl<S: BuildHasher> Nodes for HashMap<Node, Hash, S> {
    fn get(&self, node: Node) -> Option<Hash> {
        HashMap::get(self, &node).copied()
    }
}

/// Perfect subtrees whose concatenation is leaves `[0, size)`, largest first.
#[must_use]
#[allow(clippy::arithmetic_side_effects)]
pub fn decomposition(size: u64) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut start = 0_u64;
    for level in (0..64_u8).rev() {
        let width = 1_u64 << level;
        if size & width != 0 {
            nodes.push(Node {
                level,
                index: start >> level,
            });
            // The bits of `size` are disjoint, so this adds `width`.
            start |= width;
        }
    }
    nodes
}

/// Largest power of two strictly smaller than `n`, for `n >= 2`.
#[allow(clippy::arithmetic_side_effects)]
const fn split(n: u64) -> u64 {
    1_u64 << (n - 1).ilog2()
}

/// The right edge of a tree: the roots of the perfect subtrees in [`decomposition`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frontier {
    /// Number of leaves.
    size: u64,
    /// Roots of the decomposition, largest subtree first.
    hashes: Vec<Hash>,
}

impl Frontier {
    /// Right edge of the empty tree.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            size: 0,
            hashes: Vec::new(),
        }
    }

    /// Restore a right edge from the roots of [`decomposition`]`(size)`, largest first.
    /// # Errors
    /// Returns `MalformedFrontier` unless there is one hash per set bit of `size`.
    pub fn from_hashes(size: u64, hashes: Vec<Hash>) -> Result<Self, TreeError> {
        if u32::try_from(hashes.len()) != Ok(size.count_ones()) {
            return Err(TreeError::MalformedFrontier);
        }
        Ok(Self { size, hashes })
    }

    /// Number of leaves.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Roots of the decomposition, largest subtree first.
    #[must_use]
    pub fn hashes(&self) -> &[Hash] {
        &self.hashes
    }

    /// Append leaf data and return the perfect subtrees (level 1 and up) it completes.
    // Sizes are stored as `i64`, so they never reach `u64::MAX`.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn push(&mut self, data: &[u8]) -> Vec<(Node, Hash)> {
        let new_size = self.size + 1;
        let mut hash = leaf_hash(data);
        let mut completed = Vec::new();
        for level in 1..=self.size.trailing_ones() {
            let Some(left) = self.hashes.pop() else {
                break;
            };
            hash = node_hash(&left, &hash);
            let level = u8::try_from(level).unwrap_or(u8::MAX);
            completed.push((
                Node {
                    level,
                    index: (new_size >> level) - 1,
                },
                hash,
            ));
        }
        self.hashes.push(hash);
        self.size = new_size;
        completed
    }

    /// Tree root.
    #[must_use]
    pub fn root(&self) -> Hash {
        fold(&self.hashes)
    }
}

/// Fold decomposition roots (largest first) into the tree root.
fn fold(hashes: &[Hash]) -> Hash {
    let mut iter = hashes.iter().rev();
    let Some(last) = iter.next() else {
        return empty_root();
    };
    iter.fold(*last, |acc, hash| node_hash(hash, &acc))
}

/// Hash of leaves `[start, end)` per RFC 6962 `MTH`, for `start < end`.
#[allow(clippy::arithmetic_side_effects)]
fn subtree<N: Nodes + ?Sized>(nodes: &N, start: u64, end: u64) -> Result<Hash, TreeError> {
    let n = end - start;
    if n.is_power_of_two() && start.is_multiple_of(n) {
        let node = Node {
            level: u8::try_from(n.trailing_zeros()).map_err(|_| TreeError::OutOfRange)?,
            index: start / n,
        };
        return nodes.get(node).ok_or(TreeError::MissingNode(node));
    }
    let k = split(n);
    Ok(node_hash(
        &subtree(nodes, start, start + k)?,
        &subtree(nodes, start + k, end)?,
    ))
}

/// Root of the first `size` leaves.
/// # Errors
/// Returns `MissingNode` if a needed subtree hash is unavailable.
pub fn root_at<N: Nodes + ?Sized>(nodes: &N, size: u64) -> Result<Hash, TreeError> {
    let hashes = decomposition(size)
        .into_iter()
        .map(|node| nodes.get(node).ok_or(TreeError::MissingNode(node)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(fold(&hashes))
}

/// RFC 6962 audit path of leaf `index` in the tree of `size` leaves, deepest first.
/// # Errors
/// Returns `OutOfRange` unless `index < size`, or `MissingNode`.
pub fn inclusion_path<N: Nodes + ?Sized>(
    nodes: &N,
    index: u64,
    size: u64,
) -> Result<Vec<Hash>, TreeError> {
    if index >= size {
        return Err(TreeError::OutOfRange);
    }
    let mut path = Vec::new();
    path_into(nodes, index, 0, size, &mut path)?;
    Ok(path)
}

/// Recursive step of [`inclusion_path`], for `start <= index < end`.
#[allow(clippy::arithmetic_side_effects)]
fn path_into<N: Nodes + ?Sized>(
    nodes: &N,
    index: u64,
    start: u64,
    end: u64,
    path: &mut Vec<Hash>,
) -> Result<(), TreeError> {
    let n = end - start;
    if n == 1 {
        return Ok(());
    }
    let k = split(n);
    if index < start + k {
        path_into(nodes, index, start, start + k, path)?;
        path.push(subtree(nodes, start + k, end)?);
    } else {
        path_into(nodes, index, start + k, end, path)?;
        path.push(subtree(nodes, start, start + k)?);
    }
    Ok(())
}

/// RFC 6962 consistency proof from the first `old` leaves to the first `new` leaves.
/// # Errors
/// Returns `OutOfRange` unless `0 < old <= new`, or `MissingNode`.
pub fn consistency_path<N: Nodes + ?Sized>(
    nodes: &N,
    old: u64,
    new: u64,
) -> Result<Vec<Hash>, TreeError> {
    if old == 0 || old > new {
        return Err(TreeError::OutOfRange);
    }
    let mut path = Vec::new();
    if old < new {
        subproof_into(nodes, old, 0, new, true, &mut path)?;
    }
    Ok(path)
}

/// Recursive step of [`consistency_path`] (RFC 6962 `SUBPROOF`), for
/// `start < old <= end`.
#[allow(clippy::arithmetic_side_effects)]
fn subproof_into<N: Nodes + ?Sized>(
    nodes: &N,
    old: u64,
    start: u64,
    end: u64,
    complete: bool,
    path: &mut Vec<Hash>,
) -> Result<(), TreeError> {
    if old == end {
        if !complete {
            path.push(subtree(nodes, start, end)?);
        }
        return Ok(());
    }
    let k = split(end - start);
    if old <= start + k {
        subproof_into(nodes, old, start, start + k, complete, path)?;
        path.push(subtree(nodes, start + k, end)?);
    } else {
        subproof_into(nodes, old, start + k, end, false, path)?;
        path.push(subtree(nodes, start, start + k)?);
    }
    Ok(())
}

/// Records which perfect subtrees a computation reads, without knowing their hashes.
///
/// Tree shapes do not depend on hash values, so running a computation once against
/// a recorder yields exactly the nodes to fetch before running it for real.
#[derive(Default)]
pub struct Recorder(std::cell::RefCell<Vec<Node>>);

impl Recorder {
    /// Nodes requested so far, deduplicated and sorted.
    #[must_use]
    pub fn into_nodes(self) -> Vec<Node> {
        let mut nodes = self.0.into_inner();
        nodes.sort_unstable();
        nodes.dedup();
        nodes
    }
}

impl Nodes for Recorder {
    fn get(&self, node: Node) -> Option<Hash> {
        self.0.borrow_mut().push(node);
        Some([0; 32])
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;
    use crate::{ConsistencyProof, InclusionProof, LeafHash};
    use ct_merkle::mem_backed_tree::MemoryBackedTree;

    fn data(i: u64) -> Vec<u8> {
        Sha256::digest(i.to_le_bytes()).to_vec()
    }

    /// Build both trees to `size`, returning the stored nodes.
    fn build(
        size: u64,
    ) -> (
        MemoryBackedTree<Sha256, LeafHash>,
        Frontier,
        HashMap<Node, Hash>,
    ) {
        let mut reference = MemoryBackedTree::new();
        let mut frontier = Frontier::new();
        let mut nodes = HashMap::new();
        for i in 0..size {
            reference.push(LeafHash::new(data(i)));
            nodes.insert(Node { level: 0, index: i }, leaf_hash(&data(i)));
            nodes.extend(frontier.push(&data(i)));
        }
        (reference, frontier, nodes)
    }

    fn check_size(size: u64, olds: impl Iterator<Item = u64>, indices: impl Iterator<Item = u64>) {
        let (reference, frontier, nodes) = build(size);
        let root = reference.root().as_bytes().to_vec();
        assert_eq!(frontier.root().to_vec(), root, "root at {size}");
        assert_eq!(root_at(&nodes, size).unwrap().to_vec(), root);
        assert_eq!(
            nodes.keys().filter(|node| node.level > 0).count() as u64,
            size - u64::from(size.count_ones()),
            "one stored node per completed perfect subtree"
        );
        let edge: Vec<Hash> = decomposition(size)
            .into_iter()
            .map(|node| nodes[&node])
            .collect();
        assert_eq!(frontier.hashes(), edge.as_slice(), "right edge at {size}");
        assert_eq!(Frontier::from_hashes(size, edge).unwrap(), frontier);
        for index in indices {
            let ours = inclusion_path(&nodes, index, size).unwrap().concat();
            let theirs = reference.prove_inclusion(usize::try_from(index).unwrap());
            assert_eq!(ours, theirs.as_bytes(), "inclusion {index} of {size}");
            InclusionProof {
                index,
                root: root.clone(),
                proof_bytes: ours,
                tree_size: size,
            }
            .verify(&data(index))
            .unwrap();
        }
        for old in olds {
            let ours = consistency_path(&nodes, old, size).unwrap().concat();
            let theirs = reference.prove_consistency(usize::try_from(size - old).unwrap());
            assert_eq!(ours, theirs.as_bytes(), "consistency {old} -> {size}");
            ConsistencyProof {
                old_tree_size: old,
                proof_bytes: ours,
                new_root: root.clone(),
                new_tree_size: size,
            }
            .verify(&root_at(&nodes, old).unwrap(), old)
            .unwrap();
        }
    }

    #[test]
    fn matches_ct_merkle_for_every_small_tree() {
        assert_eq!(Frontier::new().root(), empty_root());
        for size in 1..=130 {
            check_size(size, 1..=size, 0..size);
        }
    }

    #[test]
    fn matches_ct_merkle_for_large_trees() {
        for size in [1_023_u64, 1_024, 1_025, 4_097, 10_007] {
            let olds = [1, 2, 3, size / 3, size / 2, size - 1, size];
            let indices = [0, 1, size / 2, size - 2, size - 1];
            check_size(size, olds.into_iter(), indices.into_iter());
        }
    }

    #[test]
    fn recorder_finds_exactly_the_needed_nodes() {
        let (_, _, nodes) = build(77);
        let recorder = Recorder::default();
        consistency_path(&recorder, 29, 77).unwrap();
        inclusion_path(&recorder, 40, 77).unwrap();
        root_at(&recorder, 29).unwrap();
        let needed: HashMap<Node, Hash> = recorder
            .into_nodes()
            .into_iter()
            .map(|node| (node, nodes[&node]))
            .collect();
        assert!(needed.len() < 30);
        assert_eq!(
            consistency_path(&needed, 29, 77),
            consistency_path(&nodes, 29, 77)
        );
        assert_eq!(
            inclusion_path(&needed, 40, 77),
            inclusion_path(&nodes, 40, 77)
        );
        assert_eq!(root_at(&needed, 29), root_at(&nodes, 29));
    }

    #[test]
    fn rejects_malformed_inputs() {
        let (_, frontier, nodes) = build(5);
        // Five leaves have two decomposition roots; seven have three.
        assert_eq!(
            Frontier::from_hashes(7, frontier.hashes().to_vec()),
            Err(TreeError::MalformedFrontier)
        );
        assert_eq!(
            Frontier::from_hashes(0, frontier.hashes().to_vec()),
            Err(TreeError::MalformedFrontier)
        );
        assert_eq!(inclusion_path(&nodes, 5, 5), Err(TreeError::OutOfRange));
        assert_eq!(consistency_path(&nodes, 0, 5), Err(TreeError::OutOfRange));
        assert_eq!(consistency_path(&nodes, 6, 5), Err(TreeError::OutOfRange));
        assert_eq!(consistency_path(&nodes, 5, 5), Ok(vec![]));
        let mut partial = nodes.clone();
        partial.remove(&Node { level: 2, index: 0 });
        assert_eq!(
            root_at(&partial, 5),
            Err(TreeError::MissingNode(Node { level: 2, index: 0 }))
        );
    }
}
