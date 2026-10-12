// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! GroupScalar implementations for P-256 group

use crate::traits::groups::GroupScalar;
use crate::utils::error::Error as CryptographyError;
use crate::utils::rng;
use core::fmt::Debug;
use p256::Scalar;
use p256::Scalar as P256CrateScalar;
use p256::elliptic_curve::Field;
use std::ops::Neg;

/**
 * A [`GroupScalar`] implementation for the P-256 group.
 */
#[derive(Debug, Clone, Copy)]
pub struct P256Scalar(pub Scalar);

impl P256Scalar {
    /// Create a new `P256Scalar` from a p256 [Scalar](https://docs.rs/p256/latest/p256/struct.Scalar.html).
    #[must_use]
    pub fn new(scalar: Scalar) -> Self {
        P256Scalar(scalar)
    }

    /// Reduce a 32-byte big-endian value into the scalar field.
    ///
    /// Unlike deserialization, this accepts any 32-byte input rather than
    /// requiring a canonical representative below the group order. That is what
    /// is needed when adopting another implementation's challenge convention:
    /// challenges are often specified as integers of a fixed bit length that may
    /// exceed the order, and reducing is sound because such values are only ever
    /// used as exponents, where `g^e = g^(e mod q)`.
    #[must_use]
    pub fn from_bytes_reduced(bytes: &[u8; 32]) -> Self {
        use p256::elliptic_curve::ops::Reduce;
        P256Scalar(<Scalar as Reduce<p256::U256>>::reduce(
            &p256::U256::from_be_slice(bytes),
        ))
    }
}

#[allow(clippy::arithmetic_side_effects)]
impl GroupScalar for P256Scalar {
    fn zero() -> Self {
        P256Scalar(Scalar::ZERO)
    }

    fn one() -> Self {
        P256Scalar(Scalar::ONE)
    }

    fn random<R: rng::CRng>(rng: &mut R) -> Self {
        let scalar = P256CrateScalar::random(rng);
        P256Scalar::new(scalar)
    }

    fn add(&self, other: &Self) -> Self {
        P256Scalar(self.0 + other.0)
    }

    fn sub(&self, other: &Self) -> Self {
        P256Scalar(self.0 - other.0)
    }

    fn mul(&self, other: &Self) -> Self {
        P256Scalar(self.0 * other.0)
    }

    fn neg(&self) -> Self {
        P256Scalar(self.0.neg())
    }

    fn inv(&self) -> Option<Self> {
        // p256::Scalar::invert returns a CtOption<Scalar>
        let inverted = self.0.invert();
        if inverted.is_some().unwrap_u8() == 1 {
            Some(P256Scalar(inverted.unwrap()))
        } else {
            None
        }
    }

    fn equals(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl From<u32> for P256Scalar {
    fn from(u: u32) -> P256Scalar {
        let scalar: P256CrateScalar = u.into();

        P256Scalar(scalar)
    }
}

impl PartialEq for P256Scalar {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}
impl Eq for P256Scalar {}

use crate::utils::serialization::{Deserializable, Serializable, take};
use p256::elliptic_curve::PrimeField;

impl Serializable for P256Scalar {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0.to_bytes());
    }
}

impl Deserializable for P256Scalar {
    fn read(input: &mut &[u8]) -> Result<Self, CryptographyError> {
        let bytes = take(input, 32)?;
        let array: [u8; 32] = bytes.try_into().expect("take returns exactly 32 bytes");

        let scalar = Scalar::from_repr(array.into()).map(P256Scalar);

        if scalar.is_some().into() {
            Ok(scalar.expect("scalar.is_some() == true"))
        } else {
            Err(CryptographyError::DeserializationError(
                "Failed to parse P256 scalar bytes".to_string(),
            ))
        }
    }
}
