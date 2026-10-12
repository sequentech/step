// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! GroupElement implementations for products

use crate::traits::groups::DistGroupOps;
use crate::traits::groups::GroupElement;
use crate::traits::groups::ReplGroupOps;

use crate::utils::error::Error as CryptographyError;
use crate::utils::rng;
use std::array;

impl<T: GroupElement, const N: usize> GroupElement for [T; N] {
    type Scalar = [T::Scalar; N];

    fn one() -> Self {
        array::from_fn(|_| T::one())
    }

    fn mul(&self, other: &Self) -> Self {
        array::from_fn(|i| self[i].mul(&other[i]))
    }

    fn inv(&self) -> Self {
        array::from_fn(|i| self[i].inv())
    }

    fn exp(&self, other: &Self::Scalar) -> Self {
        array::from_fn(|i| self[i].exp(&other[i]))
    }

    // `multi_exp` keeps the naive default. Specializing it would mean gathering
    // an exponent column out of `&[[T::Scalar; N]]`, which needs `Clone` on
    // `GroupScalar`, and no caller wants this form: batching raises a whole
    // width-N element to a *single* exponent, which is `dist_multi_exp` below.

    fn equals(&self, other: &Self) -> bool {
        for (i, item) in self.iter().enumerate() {
            let other: &T = &other[i];
            if !item.equals(other) {
                return false;
            }
        }
        true
    }

    fn random<R: rng::CRng>(rng: &mut R) -> Self {
        array::from_fn(|_| T::random(rng))
    }
}

impl<T: GroupElement, const N: usize> ReplGroupOps<[T; N]> for T {
    type Result = [T; N];

    fn repl_mul(&self, other: &[T; N]) -> Self::Result {
        std::array::from_fn(|i| self.mul(&other[i]))
    }

    fn repl_exp(&self, other: &[T::Scalar; N]) -> Self::Result {
        std::array::from_fn(|i| self.exp(&other[i]))
    }

    fn repl_equals(&self, other: &[T; N]) -> bool {
        for item in other {
            if !item.equals(self) {
                return false;
            }
        }
        true
    }
}

impl<T: GroupElement, const N: usize> DistGroupOps<T> for [T; N] {
    type Result = Self;

    fn dist_mul(&self, other: &T) -> Self::Result {
        std::array::from_fn(|i| self[i].mul(other))
    }

    fn dist_exp(&self, other: &T::Scalar) -> Self::Result {
        std::array::from_fn(|i| self[i].exp(other))
    }

    /// Gathers each component column and delegates to `T::multi_exp`, so a
    /// backend's specialized implementation is inherited here rather than
    /// bypassed. The gather copies pointers, not points.
    fn dist_multi_exp(
        bases: &[Self],
        exponents: &[T::Scalar],
    ) -> Result<Self::Result, CryptographyError> {
        if bases.len() != exponents.len() {
            return Err(CryptographyError::MismatchedMultiExpLength(
                bases.len(),
                exponents.len(),
            ));
        }

        // `array::try_from_fn` is unstable, so build then convert.
        let mut components: Vec<T> = Vec::with_capacity(N);
        for i in 0..N {
            let column: Vec<&T> = bases.iter().map(|base| &base[i]).collect();
            components.push(T::multi_exp(&column, exponents)?);
        }
        Ok(components
            .try_into()
            .unwrap_or_else(|_| unreachable!("built exactly N components")))
    }

    fn dist_equals(&self, other: &T) -> bool {
        for item in self {
            if !item.equals(other) {
                return false;
            }
        }
        true
    }
}
