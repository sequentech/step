// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verificatum's Fiat–Shamir convention, plugged into vsc's shuffle proof.
//!
//! The Terelius–Wikström algebra is shared; only the transcript differs. This
//! implements [`ShuffleChallenges`] so the same tested prover produces a proof
//! `vmnv` will accept (VMNV §8.3):
//!
//! ```text
//! s = RO_seed(rho | node(g, h, u, pk_omega, w, w'))
//! e = PRG(s) split into n_e-bit integers
//! v = RO_challenge(rho | node(leaf(s), tau^pos))
//! ```
//!
//! Both `e` and `v` are integers of a fixed bit length that may exceed the group
//! order; they are reduced into the scalar field, which is sound because they
//! are only ever exponents (`g^e = g^(e mod q)`).
//!
//! # Ordering dependency
//!
//! `v` is derived from the *seed*, not recomputed from the statement, so
//! [`batching_challenges`](ShuffleChallenges::batching_challenges) must run
//! before [`challenge`](ShuffleChallenges::challenge) — it caches the seed for
//! it. Both `Shuffler::shuffle_with` and `Shuffler::verify_with` call them in
//! that order. The cache makes a `VmnChallenges` single-use per proof and not
//! shareable across threads; construct one per shuffle.

use std::cell::RefCell;

use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::{Ciphertext, PublicKey};
use cryptography::groups::p256::element::P256Element;
use cryptography::groups::p256::scalar::P256Scalar;
use cryptography::utils::error::Error;
use cryptography::zkp::shuffle::{ShuffleChallenges, ShuffleCommitments};

use crate::wire::bytetree::ByteTree;
use crate::wire::crypto::{self, Hashfunction};

use crate::encode;

/// Verificatum's challenge derivation for a proof of a shuffle.
pub struct VmnChallenges {
    hash: Hashfunction,
    /// Global prefix rho, salting every oracle query (VMNV §9.3 step 4).
    rho: Vec<u8>,
    /// `n_e`, bit length of each batching component (`ebitlenro`).
    n_e: usize,
    /// `n_v`, bit length of the challenge (`vbitlenro`).
    n_v: usize,
    /// Ciphertext width omega, needed to widen the public key.
    width: usize,
    /// Seed from the batching query, consumed by the challenge query.
    seed: RefCell<Option<Vec<u8>>>,
}

impl VmnChallenges {
    pub fn new(hash: Hashfunction, rho: Vec<u8>, n_e: usize, n_v: usize, width: usize) -> Self {
        VmnChallenges {
            hash,
            rho,
            n_e,
            n_v,
            width,
            seed: RefCell::new(None),
        }
    }

    /// The batching seed of the last [`batching_challenges`] call, if any.
    pub fn seed(&self) -> Option<Vec<u8>> {
        self.seed.borrow().clone()
    }
}

/// Interpret `bytes` as a big-endian integer of `bits` nominal length and reduce
/// it into the scalar field.
fn scalar_from_bits(bytes: &[u8], bits: usize) -> Result<P256Scalar, Error> {
    let mut value = bytes.to_vec();
    // Mask down to exactly `bits`, mirroring VMN's `e_i = t_i mod 2^n_e`.
    let excess = bits % 8;
    if excess != 0 && !value.is_empty() {
        value[0] &= 0xFFu8 >> (8 - excess);
    }
    if value.len() > 32 {
        return Err(Error::DeserializationError(
            "challenge wider than 32 bytes is not supported for P-256".to_string(),
        ));
    }
    let mut fixed = [0u8; 32];
    fixed[32 - value.len()..].copy_from_slice(&value);
    Ok(P256Scalar::from_bytes_reduced(&fixed))
}

fn encode_err(e: anyhow::Error) -> Error {
    Error::SerializationError(format!("verificatum encoding failed: {e}"))
}

impl<const W: usize> ShuffleChallenges<P256Ctx, W> for VmnChallenges {
    fn batching_challenges(
        &self,
        generators: &[P256Element],
        pedersen_commitments: &[P256Element],
        pk: &PublicKey<P256Ctx>,
        ciphertexts: &[Ciphertext<P256Ctx, W>],
        permuted_ciphertexts: &[Ciphertext<P256Ctx, W>],
        _context: &[u8],
    ) -> Result<Vec<P256Scalar>, Error> {
        let g = encode::element_to_tree(&P256Element::generator()).map_err(encode_err)?;
        let h = encode::elements_to_tree(generators).map_err(encode_err)?;
        let u = encode::elements_to_tree(pedersen_commitments).map_err(encode_err)?;
        // The key enters the query WIDENED to omega, not as stored (VMNV §8.3
        // says "pk in C_kappa", but the verifier widens it first).
        let full_pk = encode::public_key_to_tree(&pk.y).map_err(encode_err)?;
        let wide_pk = crypto::wide_public_key(&full_pk, self.width)
            .map_err(|e| Error::SerializationError(format!("failed to widen public key: {e}")))?;
        let w = encode::ciphertexts_to_tree(ciphertexts).map_err(encode_err)?;
        let w_prime = encode::ciphertexts_to_tree(permuted_ciphertexts).map_err(encode_err)?;

        let seed = crypto::pos_seed(self.hash, &self.rho, &g, &h, &u, &wide_pk, &w, &w_prime);
        *self.seed.borrow_mut() = Some(seed.clone());

        // Expand the seed into one n_e-bit integer per ciphertext.
        let component = self.n_e.div_ceil(8);
        let stream = crypto::Prg::new(self.hash, &seed).generate(component * ciphertexts.len());
        stream
            .chunks(component)
            .map(|chunk| scalar_from_bits(chunk, self.n_e))
            .collect()
    }

    fn challenge(
        &self,
        _pk: &PublicKey<P256Ctx>,
        commitments: &ShuffleCommitments<P256Ctx, W>,
        _context: &[u8],
    ) -> Result<P256Scalar, Error> {
        let seed = self.seed.borrow().clone().ok_or_else(|| {
            Error::SerializationError(
                "VmnChallenges::challenge called before batching_challenges".to_string(),
            )
        })?;

        let tau = commitments_to_tree(commitments)?;
        let v = crypto::pos_challenge(self.hash, self.n_v, &self.rho, &seed, &tau);
        scalar_from_bits(&v, self.n_v)
    }
}

/// `tau^pos = node(B, A', B', C', D', F')` (VMNV §8.3).
///
/// Note the permutation commitment `u` is **not** part of this: VMN stores it
/// separately as `PermutationCommitment<l>.bt`, whereas vsc carries it inside
/// [`ShuffleCommitments`].
pub fn commitments_to_tree<const W: usize>(
    commitments: &ShuffleCommitments<P256Ctx, W>,
) -> Result<ByteTree, Error> {
    Ok(ByteTree::node(vec![
        encode::elements_to_tree(commitments.big_b_n()).map_err(encode_err)?,
        encode::element_to_tree(commitments.big_a_prime()).map_err(encode_err)?,
        encode::elements_to_tree(commitments.big_b_prime_n()).map_err(encode_err)?,
        encode::element_to_tree(commitments.big_c_prime()).map_err(encode_err)?,
        encode::element_to_tree(commitments.big_d_prime()).map_err(encode_err)?,
        encode::ciphertext_to_tree(commitments.big_f_prime()).map_err(encode_err)?,
    ]))
}
