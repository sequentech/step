// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Errors raised while decoding Verificatum wire data.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Error {
    /// Input ended in the middle of a byte tree.
    UnexpectedEnd,
    /// A complete byte tree was parsed but `n` bytes remained. Rejected because
    /// the encoding must be unique (the bytes feed Fiat-Shamir transcripts).
    TrailingBytes(usize),
    /// Byte-tree tag was neither `00` (node) nor `01` (leaf).
    BadTag(u8),
    /// Expected a leaf, found a node.
    ExpectedLeaf,
    /// Expected a node, found a leaf.
    ExpectedNode,
    /// A node had the wrong number of children.
    WrongArity { expected: usize, found: usize },
    /// A fixed-width integer field was not the expected width.
    WrongWidth { expected: usize, found: usize },
    /// A value did not fit the width it had to be encoded in.
    ValueTooWide,
    /// A marshalled string was not `comment::hex`, or the hex was malformed.
    BadMarshal(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnexpectedEnd => write!(f, "input ended mid byte-tree"),
            Error::TrailingBytes(n) => write!(f, "{n} trailing bytes after a complete byte tree"),
            Error::BadTag(t) => write!(f, "invalid byte-tree tag {t:#04x} (expected 00 or 01)"),
            Error::ExpectedLeaf => write!(f, "expected a leaf, found a node"),
            Error::ExpectedNode => write!(f, "expected a node, found a leaf"),
            Error::WrongArity { expected, found } => {
                write!(f, "expected a node of {expected} children, found {found}")
            }
            Error::WrongWidth { expected, found } => {
                write!(f, "expected a {expected}-byte field, found {found}")
            }
            Error::ValueTooWide => write!(f, "value does not fit the required width"),
            Error::BadMarshal(why) => write!(f, "malformed marshalled value: {why}"),
        }
    }
}

impl std::error::Error for Error {}
