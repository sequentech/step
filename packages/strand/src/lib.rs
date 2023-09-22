// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// #![doc = include_str!("../README.md")]

// #![warn(missing_docs)]

extern crate cfg_if;

/// Defines a generic interface to concrete modular arithmetic backends based on discrete log assumptions.
pub mod context;
/// Provides implementation of modular arithmetic backends based on discrete log assumptions.
pub mod backend;
/// ElGamal encryption.
pub mod elgamal;
/// Hashing.
pub(crate) mod hashing;
/// Serialization frontend. StrandVectors for parallel serialization.
#[doc(hidden)]
pub mod serialization;
/// Wikstrom proof of shuffle following [TW10](https://www.csc.kth.se/~dog/research/papers/TW10Conf.pdf). See also [HLDK17](https://arbor.bfh.ch/8269/1/HLKD17.pdf).
pub mod shuffler;
/// Distributed ElGamal threshold cryptosystem following [Pedersen91](https://link.springer.com/chapter/10.1007/3-540-46766-1_9).
/// See also [CGGI13](https://members.loria.fr/VCortier/files/Papers/WPES2013.pdf).
pub mod threshold;
/// Miscellaneous functions.
#[doc(hidden)]
pub mod util;
#[cfg(feature = "wasm")]
/// Webassembly API.
pub mod wasm;
/// Schnorr and Chaum-Pedersen zero knowledge proofs.
pub mod zkp;
// Symmetric encryption.
// #[cfg(feature = "openssl")]
// pub mod symmetric;
mod symmetric;

cfg_if::cfg_if! {
    if #[cfg(feature = "openssl")] {
        /// EcDSA digital signatures backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use signatures::openssl as signature;
        /// Random number generation backed by [OpenSSL](https://crates.io/crates/openssl).
        pub(crate) use random::openssl as rng;
        /// SHA-2 hashing backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use hashing::openssl as hash;
    }
    else if #[cfg(feature = "wasm")] {
        /// Ed25519 digital signatures backed by [dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek).
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
    }
}

/// Support for distributed Elgamal.
#[allow(dead_code)]
mod keymaker;
/// Random number generation.
#[doc(hidden)]
pub(crate) mod random;
/// Signature frontend.
mod signatures;