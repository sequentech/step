// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rand::rngs::OsRng;
use rand::rngs::StdRng;
use rand::CryptoRng;
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

/// num-bigint's `RandBigInt` trait is implemented for any `rand_core` 0.6 (rand 0.8) `Rng`.
/// StrandRng implements that trait generation too, delegating to the same OsRng-backed
/// rand 0.9 implementation above, so num_bigint can share StrandRng as its randomness source.
#[cfg(feature = "num_bigint")]
impl rand_core_06::CryptoRng for StrandRng {}

#[cfg(feature = "num_bigint")]
impl rand_core_06::RngCore for StrandRng {
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        RngCore::next_u32(self)
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        RngCore::next_u64(self)
    }

    #[inline(always)]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        RngCore::fill_bytes(self, dest)
    }

    fn try_fill_bytes(
        &mut self,
        dest: &mut [u8],
    ) -> Result<(), rand_core_06::Error> {
        RngCore::fill_bytes(self, dest);
        Ok(())
    }
}

/// The same generator, seen through `rand_core` 0.10.
///
/// curve25519-dalek 5.0, ed25519-dalek 3.0 and the released RustCrypto crates
/// take their randomness through that version's traits, while `rand` 0.9 above
/// speaks 0.9 — two versions of one interface, and a type implementing one is
/// not accepted where the other is asked for. `SigningKey::generate(&mut rng)`
/// is where that first showed up.
///
/// So `StrandRng` speaks both, exactly as it already speaks 0.6 for num-bigint
/// below. Every method delegates to the implementation above, which is `OsRng`:
/// the point of this type is that strand has **one** source of randomness, and a
/// second shim must not become a second source.
///
/// `Infallible`, because the delegates already resolve the fallibility of
/// `OsRng` — `try_*` there ends in `.expect`, so nothing here can fail that
/// would not already have panicked.
impl rand_core_010::TryRng for StrandRng {
    type Error = core::convert::Infallible;

    #[inline(always)]
    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(RngCore::next_u32(self))
    }

    #[inline(always)]
    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(RngCore::next_u64(self))
    }

    #[inline(always)]
    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        RngCore::fill_bytes(self, dst);
        Ok(())
    }
}

/// The marker that says this is fit for keys.
///
/// `Rng` and `CryptoRng` both follow from the two impls here by blanket impl —
/// `Rng` for any `TryRng<Error = Infallible>`, `CryptoRng` for any
/// `TryCryptoRng` of the same. Writing either out by hand is a conflicting
/// implementation, which is how this was found.
impl rand_core_010::TryCryptoRng for StrandRng {}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: FALSE", module_path!())
}

/// RNG based on StdRng
///
/// This is currently unused, but demonstrates how to use StdRng
/// as the underlying RNG for Strand. Unlike OsRng, StdRng
/// cannot fail to provide randomness after it has been seeded,
/// so it implements RngCore without the need for TryRngCore.
pub struct StrandStdRng(StdRng);

impl CryptoRng for StrandStdRng {}

// Unlike StrandRng, StrandStdRng can implement RngCore instead of TryRngCore.
impl RngCore for StrandStdRng {
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    #[inline(always)]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }
}
