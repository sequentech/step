// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Representations of arithmetic objects as byte trees (VMNV §6).
//!
//! # The signed-encoding trap
//!
//! VMNV §6.1 encodes a multi-precision integer `n` as `leaf(bytes_k(n))` for the
//! **smallest** `k`, and §6.1's own example encodes `-263` as `FEF9` — i.e. the
//! encoding is **two's complement / signed**, not a bare magnitude. Consequently
//! a positive value whose leading byte would have its top bit set needs an extra
//! `0x00` byte in front to stay non-negative.
//!
//! This is easy to miss and silently wrong if missed. For P-256 the field prime
//! `p` and group order `q` are both 256-bit values with the top bit set, so
//! **every coordinate and every scalar occupies 33 bytes, not 32**. It is why a
//! real `FullPublicKey.bt` is 167 bytes where a 32-byte assumption predicts 163.
//!
//! Field elements (§6.2) use a *fixed* width — the smallest `k` that can hold
//! the modulus — so that elements of a given field always serialize to the same
//! length. [`fixed_width_for_modulus_bits`] computes it.

use crate::bytetree::ByteTree;
use crate::error::{Error, Result};

/// The fixed byte width used for elements of a field/group whose modulus is
/// `modulus_bits` bits wide, under VMNV's signed encoding: `ceil((bits + 1) / 8)`.
///
/// The `+ 1` is the sign bit. For a 256-bit modulus this yields 33.
pub const fn fixed_width_for_modulus_bits(modulus_bits: usize) -> usize {
    (modulus_bits + 1).div_ceil(8)
}

/// Minimal signed width for a non-negative integer given big-endian magnitude
/// `value` (VMNV §6.1: "the smallest possible integer k").
///
/// Leading zero bytes are not significant; a leading byte with its top bit set
/// forces one extra byte. Zero encodes in a single byte.
pub fn minimal_signed_width(value: &[u8]) -> usize {
    let first = value.iter().position(|&b| b != 0);
    match first {
        None => 1, // value is zero
        Some(i) => {
            let significant = value.len() - i;
            if value[i] & 0x80 != 0 {
                significant + 1
            } else {
                significant
            }
        }
    }
}

/// Encode a non-negative integer (big-endian magnitude) in exactly `width`
/// bytes, left-padding with zeros. Errors if it does not fit *as a non-negative
/// signed value* — i.e. the top bit of the result must be clear.
pub fn encode_nonneg_fixed(value: &[u8], width: usize) -> Result<Vec<u8>> {
    if minimal_signed_width(value) > width {
        return Err(Error::ValueTooWide);
    }
    let mut out = vec![0u8; width];
    let start = width - value.len().min(width);
    // Any bytes we skip are leading zeros, guaranteed by the width check above.
    out[start..].copy_from_slice(&value[value.len() - (width - start)..]);
    Ok(out)
}

/// Encode a non-negative integer in its minimal signed width (VMNV §6.1).
pub fn encode_nonneg_minimal(value: &[u8]) -> Vec<u8> {
    let width = minimal_signed_width(value);
    // Cannot fail: width was derived from this value.
    encode_nonneg_fixed(value, width).expect("minimal width always fits")
}

/// `bytes_k(-1)` — all ones in two's complement. Used for the point at infinity
/// (VMNV §6.5).
pub fn encode_neg_one_fixed(width: usize) -> Vec<u8> {
    vec![0xFF; width]
}

/// Strip leading zero bytes from a fixed-width encoding, yielding the magnitude.
/// Zero decodes to an empty slice.
pub fn strip_leading_zeros(bytes: &[u8]) -> &[u8] {
    let first = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    &bytes[first..]
}

/// A field element as a byte tree: `leaf(bytes_k(a))` with fixed `k` (VMNV §6.2).
pub fn field_element(value: &[u8], width: usize) -> Result<ByteTree> {
    Ok(ByteTree::leaf(encode_nonneg_fixed(value, width)?))
}

/// An array of same-typed elements: `node(a_1, ..., a_l)` (VMNV §6.2, §6.6).
pub fn array(elements: Vec<ByteTree>) -> ByteTree {
    ByteTree::node(elements)
}

/// An affine curve point over a prime field: `node(leaf(x), leaf(y))` with both
/// coordinates at the field's fixed width (VMNV §6.5).
pub fn curve_point(x: &[u8], y: &[u8], width: usize) -> Result<ByteTree> {
    Ok(ByteTree::node(vec![
        field_element(x, width)?,
        field_element(y, width)?,
    ]))
}

/// The point at infinity: `node(leaf(bytes_k(-1)), leaf(bytes_k(-1)))` (VMNV §6.5).
pub fn point_at_infinity(width: usize) -> ByteTree {
    ByteTree::node(vec![
        ByteTree::leaf(encode_neg_one_fixed(width)),
        ByteTree::leaf(encode_neg_one_fixed(width)),
    ])
}

/// Whether a decoded point is the point at infinity.
pub fn is_point_at_infinity(point: &ByteTree, width: usize) -> bool {
    match point.as_node_of(2) {
        Ok(coords) => {
            let ones = encode_neg_one_fixed(width);
            coords[0].as_leaf().map(|b| b == ones).unwrap_or(false)
                && coords[1].as_leaf().map(|b| b == ones).unwrap_or(false)
        }
        Err(_) => false,
    }
}

/// Transpose an array of width-`w` product-group elements into VMNV's storage
/// order (§6.6).
///
/// This is the representation detail most likely to be implemented backwards.
/// An array of `l` elements each of `w` components is **not** stored as `l`
/// tuples; it is stored as `w` arrays, the `i`-th holding every element's `i`-th
/// component:
///
/// ```text
/// [(a1,b1), (a2,b2), (a3,b3)]  ->  node( node(a1,a2,a3), node(b1,b2,b3) )
/// ```
///
/// `rows` is the natural (element-major) order; the result is component-major.
pub fn product_array(rows: &[Vec<ByteTree>], width: usize) -> Result<ByteTree> {
    for row in rows {
        if row.len() != width {
            return Err(Error::WrongArity {
                expected: width,
                found: row.len(),
            });
        }
    }
    let columns = (0..width)
        .map(|i| ByteTree::node(rows.iter().map(|row| row[i].clone()).collect::<Vec<_>>()))
        .collect::<Vec<_>>();
    Ok(ByteTree::node(columns))
}

/// Inverse of [`product_array`]: component-major back to element-major.
pub fn product_array_rows(tree: &ByteTree) -> Result<Vec<Vec<ByteTree>>> {
    let columns = tree.as_node()?;
    let width = columns.len();
    if width == 0 {
        return Ok(Vec::new());
    }
    let len = columns[0].as_node()?.len();
    let mut rows = vec![Vec::with_capacity(width); len];
    for column in columns {
        let entries = column.as_node()?;
        if entries.len() != len {
            return Err(Error::WrongArity {
                expected: len,
                found: entries.len(),
            });
        }
        for (row, entry) in rows.iter_mut().zip(entries) {
            row.push(entry.clone());
        }
    }
    Ok(rows)
}

/// An array of booleans: `leaf(b)` with `01` for true and `00` for false
/// (VMNV §6.1). Used by `CorrectIndices.bt`.
pub fn bool_array(values: &[bool]) -> ByteTree {
    ByteTree::leaf(
        values
            .iter()
            .map(|&b| if b { 1u8 } else { 0u8 })
            .collect::<Vec<u8>>(),
    )
}

/// Decode a boolean array leaf.
pub fn bool_array_values(tree: &ByteTree) -> Result<Vec<bool>> {
    Ok(tree.as_leaf()?.iter().map(|&b| b != 0).collect())
}
