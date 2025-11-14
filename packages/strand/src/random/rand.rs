// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rand::rngs::OsRng;
use rand::CryptoRng;
// use rand::
use rand::RngCore;
use rand::TryRngCore;

/// Single source of randomness used in strand.
///
/// Random number generation provided by rand and [OsRng](https://docs.rs/rand/latest/rand/rngs/struct.OsRng.html).
/// OsRng sources randomness from the operating system via the [getrandom](https://crates.io/crates/getrandom) crate.
/// The exact implementation of the underlying rng is [OS-dependent](https://docs.rs/getrandom/latest/getrandom).
/// OsRng is [marked](https://docs.rs/rand/latest/rand/trait.CryptoRng.html) as a cryptographically secure
/// random number generator.
///
/// When building a wasm target getrandom will source randomness from
/// [Crypto.getRandomValues](https://www.w3.org/TR/WebCryptoAPI/#Crypto-method-getRandomValues) if [available](https://caniuse.com/getrandomvalues).
pub struct StrandRng;

impl CryptoRng for StrandRng {}

impl RngCore for StrandRng {
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        OsRng.try_next_u32().expect("Fixme")
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        OsRng.try_next_u64().expect("Fixme")
    }

    #[inline(always)]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        OsRng.try_fill_bytes(dest).expect("Fixme")
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}
