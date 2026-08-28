// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! GroupElement implementations for P-256 group

use crate::groups::p256::scalar::P256Scalar;
use crate::traits::groups::GroupElement;
use crate::utils::error::Error as CryptographyError;
use crate::utils::rng;
use core::fmt::Debug;
use p256::elliptic_curve::Group;
use p256::elliptic_curve::sec1::{FromSec1Point, ToSec1Point};
use p256::elliptic_curve::subtle::CtOption;
use p256::{Sec1Point, ProjectivePoint};

/**
 * A [`GroupElement`] implementation for the P-256 curve.
 */
#[derive(Debug, Clone, Copy)]
pub struct P256Element(pub ProjectivePoint);

impl P256Element {
    /// Create a new `P256Element` from a [`ProjectivePoint`](https://docs.rs/p256/latest/p256/type.ProjectivePoint.html).
    #[must_use]
    pub fn new(point: ProjectivePoint) -> Self {
        P256Element(point)
    }

    /// The standard generator of the curve.
    #[must_use]
    pub fn generator() -> Self {
        P256Element(ProjectivePoint::GENERATOR)
    }

    /// Whether this is the identity (the point at infinity).
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.0 == ProjectivePoint::IDENTITY
    }

    /// The affine `(x, y)` coordinates as fixed 32-byte big-endian values, or
    /// `None` for the identity, which has no affine representation.
    ///
    /// Provided because interoperating with other implementations generally
    /// means exchanging affine coordinates, while this type stores points
    /// projectively and serializes them compressed. Keeping the conversion here
    /// lets callers stay independent of the underlying `p256` crate.
    #[must_use]
    pub fn to_affine_xy(&self) -> Option<([u8; 32], [u8; 32])> {
        if self.is_identity() {
            return None;
        }
        // Uncompressed SEC1 is 0x04 || x(32) || y(32).
        let point = self.0.to_affine().to_sec1_point(false);
        let bytes = point.as_bytes();
        if bytes.len() != 65 || bytes[0] != 0x04 {
            return None;
        }
        let mut x = [0u8; 32];
        let mut y = [0u8; 32];
        x.copy_from_slice(&bytes[1..33]);
        y.copy_from_slice(&bytes[33..65]);
        Some((x, y))
    }

    /// Build an element from affine `(x, y)` coordinates, returning `None` if
    /// the point is not on the curve.
    #[must_use]
    pub fn from_affine_xy(x: &[u8; 32], y: &[u8; 32]) -> Option<Self> {
        let mut sec1 = [0u8; 65];
        sec1[0] = 0x04;
        sec1[1..33].copy_from_slice(x);
        sec1[33..65].copy_from_slice(y);

        let point = Sec1Point::from_bytes(&sec1).ok()?;
        let projective: Option<ProjectivePoint> = ProjectivePoint::from_sec1_point(&point).into();
        projective.map(P256Element)
    }
}

#[allow(clippy::arithmetic_side_effects)]
impl GroupElement for P256Element {
    type Scalar = P256Scalar;

    fn one() -> Self {
        P256Element(ProjectivePoint::IDENTITY)
    }

    fn random<R: rng::CRng>(rng: &mut R) -> Self {
        P256Element::new(ProjectivePoint::random(rng))
    }

    fn mul(&self, other: &Self) -> Self {
        P256Element(self.0 + other.0)
    }

    fn inv(&self) -> Self {
        P256Element(-self.0)
    }

    fn exp(&self, scalar: &Self::Scalar) -> Self {
        P256Element(self.0 * scalar.0)
    }

    fn equals(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialEq for P256Element {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}
impl Eq for P256Element {}
impl std::hash::Hash for P256Element {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_affine().to_sec1_point(true).hash(state);
    }
}

use crate::utils::serialization::{Deserializable, Serializable, take};

impl Serializable for P256Element {
    fn write(&self, out: &mut Vec<u8>) {
        // SEC1 compressed (33 bytes). The identity has no 33-byte SEC1 form
        // (SEC1 encodes it as a single zero byte), so it gets the custom,
        // unique encoding [0u8; 33].
        let point = self.0.to_affine().to_sec1_point(true);
        let bytes = point.as_bytes();
        if bytes.len() == 33 {
            out.extend_from_slice(bytes);
        } else {
            out.extend_from_slice(&[0u8; 33]);
        }
    }
}

impl Deserializable for P256Element {
    fn read(input: &mut &[u8]) -> Result<Self, CryptographyError> {
        let bytes = take(input, 33)?;
        let array: [u8; 33] = bytes.try_into().expect("take returns exactly 33 bytes");
        if array == [0u8; 33] {
            return Ok(P256Element::one());
        }

        let point = Sec1Point::from_bytes(array).map_err(|_| {
            CryptographyError::DeserializationError(
                "Failed to parse P256 encoded point".to_string(),
            )
        })?;
        let point: CtOption<P256Element> =
            ProjectivePoint::from_sec1_point(&point).map(P256Element).into();

        if point.is_some().into() {
            Ok(point.expect("point.is_some() == true"))
        } else {
            Err(CryptographyError::DeserializationError(
                "Failed to parse P256 point bytes".to_string(),
            ))
        }
    }
}
