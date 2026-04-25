// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Random number generation

use rand::rngs::{StdRng, SysRng};
use rand::SeedableRng;

/**
 * Marker trait to require a cryptographically secure random number generator.
 */
pub trait CRng: rand::Rng + rand::CryptoRng {}

/**
 * `OsRng` is a cryptographically secure random number generator.
 */
impl CRng for StdRng {}

/**
 * Random number generation [context][`crate::context::Context`] dependency.
 *
 * Allows retrieving an rng instance in some [Context][`crate::context::Context`].
 */
pub trait Rng: CRng {
    /// Returns an rng instance.
    fn rng() -> Self;
}

/**
 * Implements the random number generation [context][`crate::context::Context`] dependency with [`OsRng`].
 */
impl Rng for StdRng {
    fn rng() -> StdRng {
        // rand::rngs::StdRng
        // FIXME we will have to change our Rng trait to be fallible
        // this fallibility is present only on construction, since once StdRng is constructed, it is deterministic and will not fail 
        panic!();
        StdRng::try_from_rng(&mut SysRng).unwrap()
    }
}
