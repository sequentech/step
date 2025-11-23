// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use openssl::rand::rand_bytes;
use rand::CryptoRng;
use rand::Error;
use rand::RngCore;

/// Random number generation provided by OpenSSL (CTR_DRBG).
pub struct StrandRng;

impl CryptoRng for StrandRng {}

impl RngCore for StrandRng {
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        rand_core::impls::next_u32_via_fill(self)
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        rand_core::impls::next_u64_via_fill(self)
    }

    #[inline(always)]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand_bytes(dest).unwrap();
    }

    #[inline(always)]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        rand_bytes(dest).map_err(|e| rand::Error::new(e))
    }
}

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: TRUE", module_path!())
}
