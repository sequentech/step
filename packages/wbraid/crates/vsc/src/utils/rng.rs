// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Random number generation

use rand::rngs::ThreadRng;
use rand::{rngs::StdRng, rngs::SysRng};

/**
 * Marker trait to require a cryptographically secure random number generator.
 */
pub trait CRng: rand::Rng + rand::CryptoRng {}

/**
 * `ThreadRng` is a cryptographically secure random number generator.
 * 
 * When compiling to WebAssembly, the underlying [`getrandom`] crate sources entropy from the
 * browser's Web Crypto API (`crypto.getRandomValues`) via JS interop.  
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
 * 
 * When compiling to WebAssembly, the underlying [`getrandom`] crate sources entropy from the
 * browser's Web Crypto API (`crypto.getRandomValues`) via JS interop.  
 */
impl CRng for UnwrapErr<SysRng> {}

/**
 * Implements the random number generation [context][`crate::context::Context`] dependency with [`UnwrapErr`] and [`SysRng`].
 * 
 * Note that this will panic if the underlying `SysRng` fails to generate random bytes. In practice, `SysRng`
 * delegates directly to the OS or platform RNG (`getrandom` on Linux, `BCryptGenRandom` on Windows,
 * `crypto.getRandomValues` in browsers), and failure of these primitives is considered a non-recoverable
 * system fault. Propagating the error rather than panicking would add complexity to all call sites without
 * any meaningful recovery path.
 * 
 * If we want to use a fallible RNG in the future, we can change our Rng trait to 
 * be fallible and implement it for `SysRng` directly, without using `UnwrapErr`, for example starting
 * with the following code:
 * 
 * `pub trait CTryRng: rand::TryRng + rand::TryCryptoRng {}`
 */
impl Rng for UnwrapErr<SysRng> {
    fn rng() -> UnwrapErr<SysRng> {
        UnwrapErr(SysRng)
    }
}

/**
 * `StdRng` is a cryptographically secure random number generator.
 * 
 * When compiling to WebAssembly, the underlying [`getrandom`] crate sources entropy from the
 * browser's Web Crypto API (`crypto.getRandomValues`) via JS interop.  
 */
impl CRng for StdRng {}

/*
We cannot implement Rng for StdRng because StdRng::try_from_rng is fallible, and our Rng trait is infallible. 
Additionally, even if constructing StdRng was infallible, it would not an efficient choice, as calls to
Context::get_rng() are very frequent in small functions, and these calls would pay the cost of constructing 
a new StdRng instance every time.   

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