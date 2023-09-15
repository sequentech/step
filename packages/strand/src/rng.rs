// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only


use rand::CryptoRng;
use rand::Error;
use rand::RngCore;

/// Random number generation frontend.
pub struct StrandRng;

impl CryptoRng for StrandRng {}

cfg_if::cfg_if! {
    if #[cfg(feature = "openssl")] {
        // OpenSSL needs to be configured to operate in FIPS mode
        // https://github.com/sfackler/rust-openssl/issues/1924
        // https://www.openssl.org/docs/man3.0/man7/fips_module.html
        
        use openssl::rand::rand_bytes;

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
    }
    else {
        use rand::rngs::OsRng;
        
        impl RngCore for StrandRng {
            #[inline(always)]
            fn next_u32(&mut self) -> u32 {
                OsRng.next_u32()
            }
        
            #[inline(always)]
            fn next_u64(&mut self) -> u64 {
                OsRng.next_u64()
            }
        
            #[inline(always)]
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                OsRng.fill_bytes(dest)
            }
        
            #[inline(always)]
            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
                OsRng.try_fill_bytes(dest)
            }
        }
    }
}