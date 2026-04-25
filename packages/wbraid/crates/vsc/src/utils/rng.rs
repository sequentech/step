// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Random number generation

use rand::rngs::ThreadRng;
// StdRng
use rand::{SeedableRng, rngs::StdRng, rngs::SysRng};

/**
 * Marker trait to require a cryptographically secure random number generator.
 */
pub trait CRng: rand::Rng + rand::CryptoRng {}

/**
 * `StdRng` is a cryptographically secure random number generator.
 */
impl CRng for StdRng {}

/**
 * `ThreadRng` is a cryptographically secure random number generator.
 */
impl CRng for ThreadRng {}

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
 * Implements the random number generation [context][`crate::context::Context`] dependency with [`StdRng`].
 */
impl Rng for StdRng {
    fn rng() -> StdRng {
        // rand::rngs::StdRng
        // FIXME we would have to change our Rng trait to be fallible
        // this fallibility is present only on construction, since once StdRng is constructed, it is deterministic and will not fail 
        StdRng::try_from_rng(&mut SysRng).unwrap();
        panic!();
    }
}

/**
 * Implements the random number generation [context][`crate::context::Context`] dependency with [`ThreadRng`].
 */
impl Rng for ThreadRng {
    fn rng() -> ThreadRng {
        rand::rng()
    }
}

use rand::rand_core::UnwrapErr;

/**
 * `SysRng` is a cryptographically secure random number generator.
 */
impl CRng for UnwrapErr<SysRng> {}

/**
 * Implements the random number generation [context][`crate::context::Context`] dependency with [`UnwrapErr`] and [`SysRng`].
 */
impl Rng for UnwrapErr<SysRng> {
    fn rng() -> UnwrapErr<SysRng> {
        UnwrapErr(SysRng)
    }
}

// Fallible random number generation

/**
 * Marker trait to require a fallible, cryptographically secure random number generator.
 */
pub trait CTryRng: rand::TryRng + rand::TryCryptoRng {}

/**
 * Fallible random number generation dependency.
 * 
 * Currently a cryptographic [context][`crate::context::Context`] depends on an infallible random number generator, but this trait allows for a fallible rng to be used in the future if needed.
 *
 * Allows retrieving an fallible rng instance in some [Context][`crate::context::Context`].
 */
pub trait TRng: CTryRng {
    /// Returns an rng instance.
    fn rng() -> Self;
}

/**
 * `SysRng` is a cryptographically secure random number generator.
 */
impl CTryRng for SysRng {}

/**
 * Implements the fallible random number generation [context][`crate::context::Context`] dependency with [`SysRng`].
 * 
 * Currently a cryptographic [context][`crate::context::Context`] depends on an infallible random number generator, but this trait allows for a fallible rng to be used in the future if needed.
 */
impl TRng for SysRng {
    fn rng() -> SysRng {
        SysRng
    }
}