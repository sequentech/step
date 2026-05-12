// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use core::convert::Infallible;

use rand::rngs::SysRng;
use rand::TryCryptoRng;
use rand::TryRng;

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

impl TryCryptoRng for StrandRng {}

impl TryRng for StrandRng {
    type Error = Infallible;

    #[inline(always)]
    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(SysRng.try_next_u32().expect("Fixme"))
    }

    #[inline(always)]
    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(SysRng.try_next_u64().expect("Fixme"))
    }

    #[inline(always)]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        SysRng.try_fill_bytes(dest).expect("Fixme");
        Ok(())
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}


