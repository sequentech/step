// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Converting between vsc's P-256 types and Verificatum byte trees.
//!
//! This module is the **only** place the two type systems meet. the `wire` layer knows
//! Verificatum's wire format and nothing about vsc; vsc knows the cryptography
//! and nothing about Verificatum. Keeping the adapter here means the layer whose
//! bytes must match VMN exactly stays free of the crypto stack.
//!
//! Everything here is P-256 specific by necessity: Verificatum's `ECqPGroup`
//! supports only a fixed list of standard curves, and Ristretto255 — braid's
//! default — is not among them (see `VERIFICATUM.md`).
//!
//! # Encoding notes
//!
//! - A point is `node(leaf(x), leaf(y))` with both coordinates at the **33-byte**
//!   signed width, not 32 (`crate::wire::arithm` explains why).
//! - vsc stores points compressed (SEC1, 33 bytes); VMN wants affine `(x, y)`,
//!   so we round-trip through the uncompressed SEC1 encoding.
//! - A width-`W` ciphertext is `node(u-components, v-components)`; an **array**
//!   of them transposes each side into `W` arrays of `N` (VMNV §6.6), which is
//!   the detail most likely to be implemented backwards.

use anyhow::{anyhow, Result};

use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::groups::p256::element::P256Element;
use cryptography::groups::p256::scalar::P256Scalar;
use cryptography::traits::groups::GroupElement;

use crate::wire::arithm;
use crate::wire::bytetree::ByteTree;
use crate::wire::marshal::p256::WIDTH;

/// Encode a P-256 group element as a Verificatum affine point.
///
/// The identity maps to the point at infinity, `node(leaf(-1), leaf(-1))`
/// (VMNV §6.5).
pub fn element_to_tree(element: &P256Element) -> Result<ByteTree> {
    let Some((x, y)) = element.to_affine_xy() else {
        return Ok(arithm::point_at_infinity(WIDTH));
    };
    arithm::curve_point(&x, &y, WIDTH).map_err(|e| anyhow!("failed to encode point: {e}"))
}

/// Decode a Verificatum affine point into a P-256 group element.
pub fn tree_to_element(tree: &ByteTree) -> Result<P256Element> {
    if arithm::is_point_at_infinity(tree, WIDTH) {
        return Ok(P256Element::one());
    }

    let coords = tree
        .as_node_of(2)
        .map_err(|e| anyhow!("not an affine point: {e}"))?;
    let x = coords[0].as_leaf().map_err(|e| anyhow!("bad x: {e}"))?;
    let y = coords[1].as_leaf().map_err(|e| anyhow!("bad y: {e}"))?;

    // Coordinates arrive at the 33-byte signed width; SEC1 wants exactly 32.
    P256Element::from_affine_xy(&fixed32(x)?, &fixed32(y)?)
        .ok_or_else(|| anyhow!("coordinates are not a point on the curve"))
}

/// Narrow a signed fixed-width coordinate to the 32 bytes SEC1 expects.
fn fixed32(bytes: &[u8]) -> Result<[u8; 32]> {
    let magnitude = arithm::strip_leading_zeros(bytes);
    if magnitude.len() > 32 {
        return Err(anyhow!("coordinate exceeds 32 bytes"));
    }
    let mut out = [0u8; 32];
    out[32 - magnitude.len()..].copy_from_slice(magnitude);
    Ok(out)
}

/// Encode an array of group elements: `node(a_1, ..., a_l)` (VMNV §6.6).
pub fn elements_to_tree(elements: &[P256Element]) -> Result<ByteTree> {
    Ok(ByteTree::node(
        elements
            .iter()
            .map(element_to_tree)
            .collect::<Result<Vec<_>>>()?,
    ))
}

/// Decode an array of group elements.
pub fn tree_to_elements(tree: &ByteTree) -> Result<Vec<P256Element>> {
    tree.as_node()
        .map_err(|e| anyhow!("not an array: {e}"))?
        .iter()
        .map(tree_to_element)
        .collect()
}

/// Encode one width-`W` ciphertext as `node(u-components, v-components)`.
pub fn ciphertext_to_tree<const W: usize>(c: &Ciphertext<P256Ctx, W>) -> Result<ByteTree> {
    let (u, v) = ciphertext_parts(c);
    Ok(ByteTree::node(vec![
        elements_to_tree(&u)?,
        elements_to_tree(&v)?,
    ]))
}

/// Encode an array of width-`W` ciphertexts.
///
/// Both sides are transposed independently: the result is
/// `node(transpose(u), transpose(v))`, where each transpose is `W` arrays of `N`
/// components rather than `N` tuples of `W` (VMNV §6.6).
pub fn ciphertexts_to_tree<const W: usize>(
    ciphertexts: &[Ciphertext<P256Ctx, W>],
) -> Result<ByteTree> {
    let mut u_rows = Vec::with_capacity(ciphertexts.len());
    let mut v_rows = Vec::with_capacity(ciphertexts.len());
    for c in ciphertexts {
        let (u, v) = ciphertext_parts(c);
        u_rows.push(u.iter().map(element_to_tree).collect::<Result<Vec<_>>>()?);
        v_rows.push(v.iter().map(element_to_tree).collect::<Result<Vec<_>>>()?);
    }
    let u = arithm::product_array(&u_rows, W).map_err(|e| anyhow!("u transpose failed: {e}"))?;
    let v = arithm::product_array(&v_rows, W).map_err(|e| anyhow!("v transpose failed: {e}"))?;
    Ok(ByteTree::node(vec![u, v]))
}

/// The `(u, v)` halves of a ciphertext as element vectors.
fn ciphertext_parts<const W: usize>(
    c: &Ciphertext<P256Ctx, W>,
) -> (Vec<P256Element>, Vec<P256Element>) {
    (c.0[0].to_vec(), c.0[1].to_vec())
}

/// Decode an array of width-`W` ciphertexts (the inverse of
/// [`ciphertexts_to_tree`], undoing the transposition).
pub fn tree_to_ciphertexts<const W: usize>(
    tree: &ByteTree,
) -> Result<Vec<Ciphertext<P256Ctx, W>>> {
    let sides = tree
        .as_node_of(2)
        .map_err(|e| anyhow!("ciphertext array is not (u, v): {e}"))?;
    let u_rows =
        arithm::product_array_rows(&sides[0]).map_err(|e| anyhow!("u untranspose failed: {e}"))?;
    let v_rows =
        arithm::product_array_rows(&sides[1]).map_err(|e| anyhow!("v untranspose failed: {e}"))?;
    if u_rows.len() != v_rows.len() {
        return Err(anyhow!("u and v halves have different lengths"));
    }

    u_rows
        .iter()
        .zip(v_rows.iter())
        .map(|(u, v)| {
            if u.len() != W || v.len() != W {
                return Err(anyhow!("expected width {W}, found {}/{}", u.len(), v.len()));
            }
            let mut parts = [[P256Element::one(); W], [P256Element::one(); W]];
            for i in 0..W {
                parts[0][i] = tree_to_element(&u[i])?;
                parts[1][i] = tree_to_element(&v[i])?;
            }
            Ok(Ciphertext(parts))
        })
        .collect()
}

/// Encode a scalar as a fixed-width field element (VMNV §6.2).
pub fn scalar_to_tree(scalar: &P256Scalar) -> Result<ByteTree> {
    use cryptography::utils::serialization::FSerializable;
    let mut bytes = Vec::with_capacity(32);
    scalar.ser_into(&mut bytes);
    if bytes.len() != 32 {
        return Err(anyhow!("expected a 32-byte scalar, got {}", bytes.len()));
    }
    arithm::field_element(&bytes, WIDTH).map_err(|e| anyhow!("failed to encode scalar: {e}"))
}

/// Encode an array of scalars.
pub fn scalars_to_tree(scalars: &[P256Scalar]) -> Result<ByteTree> {
    Ok(ByteTree::node(
        scalars
            .iter()
            .map(scalar_to_tree)
            .collect::<Result<Vec<_>>>()?,
    ))
}

/// Decode a fixed-width field element into a scalar.
///
/// Values written by a conforming implementation are canonical (below the group
/// order), so the reduction is a no-op; it is used only to avoid rejecting on a
/// representation quirk.
pub fn tree_to_scalar(tree: &ByteTree) -> Result<P256Scalar> {
    let bytes = tree.as_leaf().map_err(|e| anyhow!("not a scalar: {e}"))?;
    Ok(P256Scalar::from_bytes_reduced(&fixed32(bytes)?))
}

/// Decode an array of scalars.
pub fn tree_to_scalars(tree: &ByteTree) -> Result<Vec<P256Scalar>> {
    tree.as_node()
        .map_err(|e| anyhow!("not a scalar array: {e}"))?
        .iter()
        .map(tree_to_scalar)
        .collect()
}

/// The full public key `pk = (g, y)` as VMN stores it in `FullPublicKey.bt`.
pub fn public_key_to_tree(y: &P256Element) -> Result<ByteTree> {
    Ok(ByteTree::node(vec![
        element_to_tree(&P256Element::generator())?,
        element_to_tree(y)?,
    ]))
}

/// Encode an array of width-`W` elements of the plaintext group `M_{κ,ω}`.
///
/// The shape of `Plaintexts.bt` and of each `DecryptionFactors<l>.bt`: `W`
/// arrays of `N` components, transposed as in [`ciphertexts_to_tree`], but with
/// only one side rather than a `(u, v)` pair.
pub fn component_array_to_tree<const W: usize>(
    elements: &[[P256Element; W]],
) -> Result<ByteTree> {
    let rows = elements
        .iter()
        .map(|e| e.iter().map(element_to_tree).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    arithm::product_array(&rows, W).map_err(|e| anyhow!("transpose failed: {e}"))
}

/// Decode an array of width-`W` plaintext-group elements, undoing the
/// transposition (the inverse of [`component_array_to_tree`]).
///
/// Reads `Plaintexts.bt` and each `DecryptionFactors<l>.bt`.
pub fn tree_to_component_array<const W: usize>(tree: &ByteTree) -> Result<Vec<[P256Element; W]>> {
    arithm::product_array_rows(tree)
        .map_err(|e| anyhow!("untranspose failed: {e}"))?
        .iter()
        .map(|row| {
            if row.len() != W {
                return Err(anyhow!("expected width {W}, found {}", row.len()));
            }
            let mut components = [P256Element::one(); W];
            for (slot, component) in components.iter_mut().zip(row) {
                *slot = tree_to_element(component)?;
            }
            Ok(components)
        })
        .collect()
}
