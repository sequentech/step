// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! GroupElement implementations for Ristrett255 group

use crate::groups::ristretto255::scalar::RistrettoScalar;
use crate::traits::groups::GroupElement;
use crate::utils::error::Error as CryptographyError;
use crate::utils::rng;
use core::fmt::Debug;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::traits::{Identity, MultiscalarMul};
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

    /// Straus' algorithm via curve25519-dalek's [`MultiscalarMul`], rather than
    /// the trait's naive per-base default.
    ///
    /// Constant-time in the scalars, matching [`Self::exp`] and the contract on
    /// [`GroupElement::multi_exp`]. dalek also offers a variable-time
    /// implementation which is faster still, but it is only sound for public
    /// scalars, so adopting it would need a separate method with that
    /// precondition in its name.
    fn multi_exp(
        bases: &[&Self],
        exponents: &[Self::Scalar],
    ) -> Result<Self, CryptographyError> {
        if bases.len() != exponents.len() {
            return Err(CryptographyError::MismatchedMultiExpLength(
                bases.len(),
                exponents.len(),
            ));
        }
        Ok(RistrettoElement(RistrettoPoint::multiscalar_mul(
            exponents.iter().map(|s| s.0),
            bases.iter().map(|b| b.0),
        )))
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
