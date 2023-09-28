// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// #![doc = include_str!("../README.md")]

extern crate cfg_if;

/// Defines a generic interface to concrete modular arithmetic backends based on
/// discrete log assumptions.
pub mod context;
/// Provides implementation of modular arithmetic backends based on discrete log
/// assumptions.
pub mod backend;

/// ElGamal encryption.
pub mod elgamal;
/// Wikstrom proof of shuffle following [TW10](https://www.csc.kth.se/~dog/research/papers/TW10Conf.pdf). See also [HLDK17](https://arbor.bfh.ch/8269/1/HLKD17.pdf).
pub mod shuffler;
/// Distributed ElGamal threshold cryptosystem following [Pedersen91](https://link.springer.com/chapter/10.1007/3-540-46766-1_9).
/// See also [CGGI13](https://members.loria.fr/VCortier/files/Papers/WPES2013.pdf).
pub mod threshold;
/// Schnorr and Chaum-Pedersen zero knowledge proofs.
pub mod zkp;

/// Random number generation.
mod random;
/// Signature frontend.
mod signatures;
/// Symmetric encryption frontend.
mod symmetric;
/// Hashing.
mod hashing;

cfg_if::cfg_if! {
    if #[cfg(feature = "openssl")] {
        /// EcDSA digital signatures backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use signatures::openssl as signature;
        /// Random number generation backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use random::openssl as rng;
        /// SHA-2 hashing backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use hashing::openssl as hash;
        /// AES-GCM backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use symmetric::openssl as symm;
    }
    else if #[cfg(feature = "wasm")] {
        /// Webassembly API.
        pub mod wasm;
        // TODO choose which signatures to use in wasm
        /// Ed25519 digital signatures backed by [dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek).
        // pub use signatures::dalek as signature;
        /// EcDSA digital signatures backed by [rustcrypto](https://docs.rs/ecdsa/latest/ecdsa/).
        pub use signatures::rustcrypto as signature;
        /// Random number generation backed by [rand](https://crates.io/crates/rand).
        pub(crate) use random::rand as rng;
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
mod keymaker;
