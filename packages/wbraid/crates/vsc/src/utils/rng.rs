// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Random number generation

use rand::rngs::ThreadRng;
// StdRng
use rand::{rngs::StdRng, rngs::SysRng};

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

/*
We cannot implement Rng for StdRng because StdRng::try_from_rng is fallible, and our Rng trait is infallible. 
Additionally, even if constructing StdRng was infallible, it would not an efficient choice, as calls to
Context::get_rng() are very frequent in small functions.

For now, we will use ThreadRng as the default rng implementation, which is infallible and cryptographically secure. 
If we want to use StdRng in the future, we can change our Rng trait to be fallible and use some kind of thread
local storage to store the StdRng instance, so that we only pay the cost of constructing it once per thread. Unlike
using `ThreadRng`, this approach would allow us to control the seed of the StdRng instance, which can yield deterministic 
behaviour for testing and debugging purposes.
 
Implements the random number generation [context][`crate::context::Context`] dependency with [`StdRng`].

impl Rng for StdRng {
    fn rng() -> StdRng {
        // rand::rngs::StdRng
        // FIXME we would have to change our Rng trait to be fallible
        // this fallibility is present only on construction, since once StdRng is constructed, it is deterministic and will not fail 
        StdRng::try_from_rng(&mut SysRng).unwrap()
    }
}*/

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
 * 
 * Note that this will panic if the underlying `SysRng` fails to generate random bytes. It is unclear at this time whether
 * this is a practical concern; for example, in the `ThreadRng` implementation, the following is stated in the documentation:
 * 
 * "Implementations of `TryRng` and Rng panic in case of `SysRng` failure during reseeding (highly unlikely)."
 * 
 * If this is not acceptable, we would have to use the `CTryRng` trait below, which would require changes to all
 * uses of the Rng to handle the potential failure. The benefit of this is questionable, since there is no
 * way to recover from an rng failure anyway.
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
 * Currently a cryptographic [context][`crate::context::Context`] depends on an infallible random number 
 * generator, but this trait allows for a fallible rng to be used in the future if needed.
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
 * Currently a cryptographic [context][`crate::context::Context`] depends on an infallible random number generator, 
 * but this trait allows for a fallible rng to be used in the future if needed.
 */
impl TRng for SysRng {
    fn rng() -> SysRng {
        SysRng
    }
}