// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// #![doc = include_str!("../README.md")]
extern crate cfg_if;

/// Provides implementation of modular arithmetic backends based on discrete log
/// assumptions.
pub mod backend;
/// Defines a generic interface to concrete modular arithmetic backends based on
/// discrete log assumptions.
pub mod context;

/// ElGamal encryption.
pub mod elgamal;
/// Wikstrom proof of shuffle following [TW10](https://www.csc.kth.se/~dog/research/papers/TW10Conf.pdf). See also [HLDK17](https://arbor.bfh.ch/8269/1/HLKD17.pdf).
#[cfg(any(test, not(feature = "wasm")))]
pub mod shuffler;
/// Distributed ElGamal threshold cryptosystem following [Pedersen91](https://link.springer.com/chapter/10.1007/3-540-46766-1_9).
/// See also [CGGI13](https://members.loria.fr/VCortier/files/Papers/WPES2013.pdf).
#[cfg(any(test, not(feature = "wasm")))]
pub mod threshold;
/// Schnorr and Chaum-Pedersen zero knowledge proofs.
pub mod zkp;

/// Hashing.
mod hashing;
/// Random number generation.
mod random;
/// Signature frontend.
mod signatures;
/// Symmetric encryption frontend.
#[cfg(not(feature = "wasm"))]
mod symmetric;

cfg_if::cfg_if! {
    if #[cfg(feature = "openssl_core")] {
        /// Random number generation backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use random::openssl as rng;
        /// SHA-2 hashing backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use hashing::openssl as hash;
        /// AES-GCM backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use symmetric::openssl as symm;
        /// Ed25519 digital signatures backed by [dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek).
        pub use signatures::dalek as signature;
    }
    else if #[cfg(feature = "openssl_full")] {
        pub use random::openssl as rng;
        /// SHA-2 hashing backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use hashing::openssl as hash;
        /// AES-GCM backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use symmetric::openssl as symm;
        /// EcDSA digital signatures backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use signatures::openssl as signature;
    }
    else if #[cfg(feature = "wasm")] {
        /// Webassembly API.
        pub mod wasm;
        /// Ed25519 digital signatures backed by [dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek).
        pub use signatures::dalek as signature;
        /// Random number generation backed by [rand](https://crates.io/crates/rand).
        pub use random::rand as rng;
        /// SHA-2 hashing backed by [rustcrypto](https://crates.io/crates/sha2).
        pub use hashing::rustcrypto as hash;
    }
    else {
        /// Ed25519 digital signatures backed by [dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek).
        pub use signatures::dalek as signature;
        /// Random number generation backed by [rand](https://crates.io/crates/rand).
        pub use random::rand as rng;
        /// SHA-2 hashing backed by [rustcrypto](https://crates.io/crates/sha2).
        pub use hashing::rustcrypto as hash;
        /// Chacha20poly1305 backed by [rustcrypto](https://docs.rs/chacha20poly1305/latest/chacha20poly1305/).
        pub use symmetric::rustcrypto as symm;
    }
}

/// Miscellaneous functions.
#[doc(hidden)]
pub mod util;

/// Serialization frontend. StrandVectors for parallel serialization.
#[doc(hidden)]
pub mod serialization;

/// Support for distributed Elgamal.
#[allow(dead_code)]
#[cfg(test)]
mod keymaker;

use std::collections::HashMap;

pub fn info() -> HashMap<&'static str, String> {
    let mut info = HashMap::new();
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
    let hash = crate::hash::info();
    let random = crate::rng::info();
    let signature = crate::signature::info();
    #[cfg(not(feature = "wasm"))]
    let symmetric = crate::symm::info();

    info.insert("VERSION", version.to_string());
    info.insert("HASH", hash);
    info.insert("RNG", random);
    info.insert("SIGNATURE", signature);
    #[cfg(not(feature = "wasm"))]
    info.insert("SYMMETRIC", symmetric);

    info
}

pub fn info_string() -> String {
    let info = info();
    let ret = format!(
        "
===============================================================================
strand

Version:    {}
HASH:       {}
RNG:        {}
SIGNATURE:  {}
SYMMETRIC:  {}
===============================================================================
",
        info.get("VERSION").unwrap(),
        info.get("HASH").unwrap(),
        info.get("RNG").unwrap(),
        info.get("SIGNATURE").unwrap(),
        info.get("SYMMETRIC").unwrap()
    );

    ret
}
