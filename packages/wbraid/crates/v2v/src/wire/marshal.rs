// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Marshalling groups (VMNV §6.7) and the P-256 parameters.
//!
//! A group is carried across the wire as
//! `node(leaf("<java class name>"), <group description>)`, hex-encoded and
//! prefixed with a human-readable comment separated by a double colon:
//!
//! ```text
//! ECqPGroup(P-256)::000000000201000000206...
//! ```
//!
//! The comment is decorative — `unmarshal` drops everything up to and including
//! the `::`. Named elliptic curve groups describe themselves with just their
//! name, so the whole descriptor is two leaves.

use crate::wire::bytetree::ByteTree;
use crate::wire::error::{Error, Result};

/// Java class name of Verificatum's standard-curve group implementation.
pub const ECQ_PGROUP_CLASS: &str = "com.verificatum.arithm.ECqPGroup";
/// Java class name of Verificatum's multiplicative-group implementation.
pub const MODP_GROUP_CLASS: &str = "com.verificatum.arithm.ModPGroup";

/// Build the marshalled byte tree for a standard named curve (VMNV §6.7).
pub fn named_curve_tree(curve: &str) -> ByteTree {
    ByteTree::node(vec![
        ByteTree::leaf(ECQ_PGROUP_CLASS.as_bytes().to_vec()),
        ByteTree::leaf(curve.as_bytes().to_vec()),
    ])
}

/// Render a byte tree in VMN's marshalled form, `comment::hex`.
pub fn marshal(comment: &str, tree: &ByteTree) -> String {
    format!("{comment}::{}", to_hex(&tree.to_bytes()))
}

/// Parse a marshalled `comment::hex` string back into a byte tree, discarding
/// the comment. Accepts a bare hex string too (no comment).
pub fn unmarshal(s: &str) -> Result<ByteTree> {
    let hex = match s.rfind("::") {
        Some(i) => &s[i + 2..],
        None => s,
    };
    let bytes = from_hex(hex.trim())?;
    ByteTree::from_bytes(&bytes)
}

/// The curve name from a marshalled `ECqPGroup` descriptor.
pub fn curve_name(tree: &ByteTree) -> Result<String> {
    let children = tree.as_node_of(2)?;
    let class = children[0].as_leaf()?;
    if class != ECQ_PGROUP_CLASS.as_bytes() {
        return Err(Error::BadMarshal("not an ECqPGroup descriptor"));
    }
    String::from_utf8(children[1].as_leaf()?.to_vec())
        .map_err(|_| Error::BadMarshal("curve name is not UTF-8"))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn from_hex(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        return Err(Error::BadMarshal("odd-length hex"));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| Error::BadMarshal("bad hex digit"))
        })
        .collect()
}

/// NIST P-256 parameters, in the form this crate needs.
///
/// Both the field prime and the group order are 256-bit with the top bit set,
/// so [`WIDTH`](p256::WIDTH) is 33 under VMNV's signed encoding — see
/// [`crate::wire::arithm`].
pub mod p256 {
    use super::*;
    use crate::wire::arithm::fixed_width_for_modulus_bits;

    /// VMN's name for this curve.
    pub const NAME: &str = "P-256";

    /// Bit length of the field prime and of the group order.
    pub const MODULUS_BITS: usize = 256;

    /// Fixed byte width of a coordinate or scalar: 33, not 32.
    pub const WIDTH: usize = fixed_width_for_modulus_bits(MODULUS_BITS);

    /// x-coordinate of the standard generator.
    pub const GENERATOR_X: [u8; 32] = [
        0x6b, 0x17, 0xd1, 0xf2, 0xe1, 0x2c, 0x42, 0x47, 0xf8, 0xbc, 0xe6, 0xe5, 0x63, 0xa4, 0x40,
        0xf2, 0x77, 0x03, 0x7d, 0x81, 0x2d, 0xeb, 0x33, 0xa0, 0xf4, 0xa1, 0x39, 0x45, 0xd8, 0x98,
        0xc2, 0x96,
    ];

    /// y-coordinate of the standard generator.
    pub const GENERATOR_Y: [u8; 32] = [
        0x4f, 0xe3, 0x42, 0xe2, 0xfe, 0x1a, 0x7f, 0x9b, 0x8e, 0xe7, 0xeb, 0x4a, 0x7c, 0x0f, 0x9e,
        0x16, 0x2b, 0xce, 0x33, 0x57, 0x6b, 0x31, 0x5e, 0xce, 0xcb, 0xb6, 0x40, 0x68, 0x37, 0xbf,
        0x51, 0xf5,
    ];

    /// The marshalled descriptor VMN writes into a protocol info file.
    pub fn group_tree() -> ByteTree {
        named_curve_tree(NAME)
    }

    /// The generator as a byte tree.
    pub fn generator() -> ByteTree {
        crate::wire::arithm::curve_point(&GENERATOR_X, &GENERATOR_Y, WIDTH)
            .expect("P-256 generator fits in the fixed width")
    }
}
