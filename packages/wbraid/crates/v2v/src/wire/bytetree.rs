// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Byte trees — Verificatum's universal serialization format (VMNV §4).
//!
//! A byte tree is either a **leaf** holding raw bytes or a **node** holding an
//! ordered list of children:
//!
//! ```text
//! leaf(d)              ->  01 || len_4(|d|)  || d
//! node(c_1, ..., c_n)  ->  00 || len_4(n)    || c_1 || ... || c_n
//! ```
//!
//! Lengths and counts are 4-byte big-endian. Everything Verificatum writes to a
//! proof directory is a byte tree, so this is the foundation the rest of the
//! crate builds on.
//!
//! Parsing is **strict**: a buffer with trailing bytes after one complete tree
//! is rejected, as is any truncated or over-long length prefix. That matters
//! because these bytes are hashed into Fiat–Shamir transcripts — a parser that
//! accepted two encodings of the same value would be a malleability surface.

use crate::wire::error::{Error, Result};

/// Tag byte introducing a leaf.
const LEAF_TAG: u8 = 0x01;
/// Tag byte introducing a node.
const NODE_TAG: u8 = 0x00;

/// A Verificatum byte tree (VMNV §4.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ByteTree {
    Leaf(Vec<u8>),
    Node(Vec<ByteTree>),
}

impl ByteTree {
    /// A leaf holding `bytes`.
    pub fn leaf(bytes: impl Into<Vec<u8>>) -> Self {
        ByteTree::Leaf(bytes.into())
    }

    /// A node holding `children`.
    pub fn node(children: impl Into<Vec<ByteTree>>) -> Self {
        ByteTree::Node(children.into())
    }

    /// Serialized length in bytes, without allocating.
    ///
    /// Used by the size-prediction tests, and lets [`to_bytes`](Self::to_bytes)
    /// allocate exactly once.
    pub fn serialized_len(&self) -> usize {
        match self {
            ByteTree::Leaf(bytes) => 1 + 4 + bytes.len(),
            ByteTree::Node(children) => {
                1 + 4 + children.iter().map(ByteTree::serialized_len).sum::<usize>()
            }
        }
    }

    /// Serialize to the byte-tree wire format (VMNV §4.2).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.serialized_len());
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut Vec<u8>) {
        match self {
            ByteTree::Leaf(bytes) => {
                out.push(LEAF_TAG);
                out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                out.extend_from_slice(bytes);
            }
            ByteTree::Node(children) => {
                out.push(NODE_TAG);
                out.extend_from_slice(&(children.len() as u32).to_be_bytes());
                for child in children {
                    child.write(out);
                }
            }
        }
    }

    /// Parse exactly one byte tree from `bytes`, rejecting trailing data.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (tree, rest) = Self::parse(bytes)?;
        if !rest.is_empty() {
            return Err(Error::TrailingBytes(rest.len()));
        }
        Ok(tree)
    }

    /// Parse one byte tree off the front of `bytes`, returning it and the
    /// remainder.
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8])> {
        let (&tag, rest) = bytes.split_first().ok_or(Error::UnexpectedEnd)?;
        if rest.len() < 4 {
            return Err(Error::UnexpectedEnd);
        }
        let (len_bytes, mut rest) = rest.split_at(4);
        let len =
            u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        match tag {
            LEAF_TAG => {
                if rest.len() < len {
                    return Err(Error::UnexpectedEnd);
                }
                let (data, tail) = rest.split_at(len);
                Ok((ByteTree::Leaf(data.to_vec()), tail))
            }
            NODE_TAG => {
                // A node claiming more children than there are remaining bytes
                // cannot be valid; reject early rather than allocating for it.
                if len > rest.len() {
                    return Err(Error::UnexpectedEnd);
                }
                let mut children = Vec::with_capacity(len);
                for _ in 0..len {
                    let (child, tail) = Self::parse(rest)?;
                    children.push(child);
                    rest = tail;
                }
                Ok((ByteTree::Node(children), rest))
            }
            other => Err(Error::BadTag(other)),
        }
    }

    /// The leaf's bytes, or an error if this is a node.
    pub fn as_leaf(&self) -> Result<&[u8]> {
        match self {
            ByteTree::Leaf(bytes) => Ok(bytes),
            ByteTree::Node(_) => Err(Error::ExpectedLeaf),
        }
    }

    /// The node's children, or an error if this is a leaf.
    pub fn as_node(&self) -> Result<&[ByteTree]> {
        match self {
            ByteTree::Node(children) => Ok(children),
            ByteTree::Leaf(_) => Err(Error::ExpectedNode),
        }
    }

    /// The node's children, requiring exactly `n` of them.
    pub fn as_node_of(&self, n: usize) -> Result<&[ByteTree]> {
        let children = self.as_node()?;
        if children.len() != n {
            return Err(Error::WrongArity {
                expected: n,
                found: children.len(),
            });
        }
        Ok(children)
    }
}
