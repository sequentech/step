// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Independent generators, as vsc group elements.
//!
//! `vcompat` derives them per VMNV §6.8 and returns byte trees; this turns those
//! into the `P256Element`s the shuffler needs.
//!
//! A shuffle emitted for Verificatum **must** use these rather than vsc's own
//! `ind_generators`: `h` feeds both the Pedersen commitments and the batching
//! seed, so a verifier deriving it Verificatum's way would reject anything else.

use anyhow::{anyhow, Result};

use cryptography::groups::p256::element::P256Element;

use vcompat::crypto::Hashfunction;
use vcompat::generators::{independent_generators, CurveParams};

use super::encode;

/// Derive `count` independent generators for P-256, salted with the global
/// prefix `rho` (VMNV §6.8, §8.2).
///
/// `n_r` is the statistical-distance parameter (`statdist` in the protocol info
/// file).
pub fn vmn_generators(
    hash: Hashfunction,
    rho: &[u8],
    n_r: usize,
    count: usize,
) -> Result<Vec<P256Element>> {
    let tree = independent_generators(hash, rho, &CurveParams::p256(), n_r, count)
        .map_err(|e| anyhow!("failed to derive independent generators: {e}"))?;
    encode::tree_to_elements(&tree)
}
