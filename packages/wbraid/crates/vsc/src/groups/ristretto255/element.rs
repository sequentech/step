// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! GroupElement implementations for Ristrett255 group

use crate::groups::ristretto255::scalar::RistrettoScalar;
use crate::traits::groups::GroupElement;
use crate::utils::error::Error as CryptographyError;
use crate::utils::rng;
use core::fmt::Debug;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoBasepointTable, RistrettoPoint};
use curve25519_dalek::traits::{Identity, MultiscalarMul, VartimeMultiscalarMul};
use rayon::prelude::*;
use sha3::digest::Digest;
use sha3::digest::typenum::U64;

/**
 * A [`GroupElement`] implementation for the [Ristretto](https://docs.rs/curve25519-dalek/latest/curve25519_dalek/ristretto/index.html) group.
 */
#[derive(Copy, Clone, Debug)]
pub struct RistrettoElement(pub RistrettoPoint);

impl RistrettoElement {
    /// Create a new `RistrettoElement` from a [`RistrettoPoint`](https://docs.rs/curve25519-dalek/latest/curve25519_dalek/ristretto/struct.RistrettoPoint.html).
    #[must_use]
    pub fn new(point: RistrettoPoint) -> Self {
        RistrettoElement(point)
    }

    /// Create a new `RistrettoElement` from a hash.
    ///
    /// See [`RistrettoPoint::hash_from_bytes`](https://docs.rs/curve25519-dalek/latest/curve25519_dalek/ristretto/struct.RistrettoPoint.html#method.hash_from_bytes) for details
    pub fn from_hash<D: Digest<OutputSize = U64> + Default>(hasher: D) -> Self {
        RistrettoElement(RistrettoPoint::from_hash::<D>(hasher))
    }
}

/// Chunk size for the parallel multiscalar-multiplication overrides.
///
/// Splits `n` bases into at most `num_threads` chunks — the strategy the
/// `msm_strategy` bench selected (a single dalek call is single-threaded and
/// loses to the already-parallel naive product; chunking by thread count wins
/// on both the constant-time and variable-time paths). `MIN_CHUNK` keeps tiny
/// inputs from being split into pathologically small chunks (below it, this
/// collapses to a single dalek call).
///
/// Depends only on `n`, never on the scalars, so it is safe to use from the
/// constant-time [`multi_exp`](RistrettoElement::multi_exp).
fn msm_chunk_size(n: usize) -> usize {
    const MIN_CHUNK: usize = 64;
    let threads = rayon::current_num_threads().max(1);
    n.div_ceil(threads).max(MIN_CHUNK)
}

/// Group addition as a named combiner for `reduce` over chunk partial products.
#[allow(clippy::arithmetic_side_effects)] // curve arithmetic
fn point_add(a: RistrettoPoint, b: RistrettoPoint) -> RistrettoPoint {
    a + b
}

impl GroupElement for RistrettoElement {
    type Scalar = RistrettoScalar;

    #[inline]
    fn one() -> Self {
        RistrettoElement(RistrettoPoint::identity())
    }

    #[inline]
    fn random<R: rng::CRng>(rng: &mut R) -> Self {
        let ret = RistrettoPoint::random(rng);
        RistrettoElement(ret)
    }

    #[inline]
    fn mul(&self, other: &Self) -> Self {
        // curve arithmetic
        #[allow(clippy::arithmetic_side_effects)]
        RistrettoElement(self.0 + other.0)
    }

    #[inline]
    fn inv(&self) -> Self {
        // curve arithmetic
        #[allow(clippy::arithmetic_side_effects)]
        RistrettoElement(-self.0)
    }

    #[inline]
    fn exp(&self, scalar: &Self::Scalar) -> Self {
        // curve arithmetic
        #[allow(clippy::arithmetic_side_effects)]
        RistrettoElement(self.0 * scalar.0)
    }

    /// Constant-time multiscalar multiplication: dalek's Straus
    /// (`MultiscalarMul`), split into `num_threads` chunks run on the rayon
    /// pool with their partial products summed.
    ///
    /// Constant-time in the scalars, matching [`Self::exp`] and the contract on
    /// [`GroupElement::multi_exp`], so it is sound for the prover's secret
    /// blinding scalars. The chunking preserves that: chunk boundaries depend
    /// only on the input length (see `msm_chunk_size`), never on the scalars,
    /// and each chunk's Straus is itself constant-time — so neither the work
    /// done nor the number of partial-sum additions varies with the exponents.
    ///
    /// Chunking is not a detail: a *single* Straus call is measured ~3.9x
    /// slower than the naive per-base product already parallel across the pool
    /// (`benches/msm_strategy.rs`), because it is single-threaded. Chunked, it
    /// is ~2.4x faster than that baseline. For public scalars use the faster
    /// [`vartime_multi_exp`](GroupElement::vartime_multi_exp).
    fn multi_exp(bases: &[&Self], exponents: &[Self::Scalar]) -> Result<Self, CryptographyError> {
        if bases.len() != exponents.len() {
            return Err(CryptographyError::MismatchedMultiExpLength(
                bases.len(),
                exponents.len(),
            ));
        }
        let chunk = msm_chunk_size(bases.len());
        Ok(RistrettoElement(
            bases
                .par_chunks(chunk)
                .zip(exponents.par_chunks(chunk))
                .map(|(bc, sc)| {
                    RistrettoPoint::multiscalar_mul(sc.iter().map(|s| s.0), bc.iter().map(|b| b.0))
                })
                .reduce(RistrettoPoint::identity, point_add),
        ))
    }

    /// Variable-time multiscalar multiplication: dalek's size-dispatched
    /// Straus/Pippenger (`VartimeMultiscalarMul`), chunked across the pool like
    /// [`multi_exp`](Self::multi_exp).
    ///
    /// **Public scalars only** — the timing depends on them (see the trait
    /// contract). Measured ~8x faster than the naive parallel product at
    /// N = 1e5 (`benches/msm_strategy.rs`); this is the verifier's path, where
    /// the batching values and published responses are all public.
    fn vartime_multi_exp(
        bases: &[&Self],
        exponents: &[Self::Scalar],
    ) -> Result<Self, CryptographyError> {
        if bases.len() != exponents.len() {
            return Err(CryptographyError::MismatchedMultiExpLength(
                bases.len(),
                exponents.len(),
            ));
        }
        let chunk = msm_chunk_size(bases.len());
        Ok(RistrettoElement(
            bases
                .par_chunks(chunk)
                .zip(exponents.par_chunks(chunk))
                .map(|(bc, sc)| {
                    RistrettoPoint::vartime_multiscalar_mul(
                        sc.iter().map(|s| s.0),
                        bc.iter().map(|b| b.0),
                    )
                })
                .reduce(RistrettoPoint::identity, point_add),
        ))
    }

    /// Fixed-base batch: one precomputed [`RistrettoBasepointTable`] over
    /// `self`, then a constant-time table multiply per scalar, in parallel.
    ///
    /// Constant-time in the scalars (dalek's basepoint-table multiply is), so
    /// it is sound for the prover's secret `g^{r_i}` / `h_1^{p_i}` batches.
    /// The table build (~one full exponentiation) amortizes across the batch,
    /// and each multiply is a fraction of a variable-base [`exp`](Self::exp).
    fn exp_many(&self, exponents: &[Self::Scalar]) -> Vec<Self> {
        let table = RistrettoBasepointTable::create(&self.0);
        exponents
            .par_iter()
            .map(|s| {
                // curve arithmetic
                #[allow(clippy::arithmetic_side_effects)]
                RistrettoElement(&table * &s.0)
            })
            .collect()
    }

    #[inline]
    fn equals(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialEq for RistrettoElement {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}
impl Eq for RistrettoElement {}
impl std::hash::Hash for RistrettoElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.compress().hash(state);
    }
}

use crate::utils::serialization::{Deserializable, Serializable, take};

impl Serializable for RistrettoElement {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0.compress().to_bytes());
    }
}

impl Deserializable for RistrettoElement {
    fn read(input: &mut &[u8]) -> Result<Self, CryptographyError> {
        let bytes = take(input, 32)?;
        let array: [u8; 32] = bytes.try_into().expect("take returns exactly 32 bytes");
        CompressedRistretto(array)
            .decompress()
            .map(RistrettoElement)
            .ok_or(CryptographyError::DeserializationError(
                "Failed to parse Ristretto point bytes".to_string(),
            ))
    }
}
