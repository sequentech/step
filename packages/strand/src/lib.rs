// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#![doc = include_str!("../README.md")]

// #![warn(missing_docs)]

extern crate cfg_if;

/// Provides cryptographic backends, currently multiplicative groups and
/// ristretto elliptic curve.
pub mod backend;
/// Defines a generic interface to concrete backends.
pub mod context;
/// ElGamal encryption.
pub mod elgamal;
/// Hashing.
#[doc(hidden)]
pub(crate) mod hashing;
/// Support for distributed Elgamal.
#[allow(dead_code)]
mod keymaker;
/// Random number generation frontend..
#[doc(hidden)]
pub(crate) mod random;
/// Serialization frontend. StrandVectors for parallel serialization.
#[doc(hidden)]
pub mod serialization;
/// Wikstrom proof of shuffle.
#[doc(hidden)]
pub mod shuffler;
/// Signature frontend.
mod signatures;
/// Support for threshold ElGamal.
#[doc(hidden)]
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
#[cfg(feature = "openssl")]
pub mod symmetric;

cfg_if::cfg_if! {
    if #[cfg(feature = "openssl")] {
        pub use signatures::openssl as signature;
        pub(crate) use random::openssl as rng;
        pub use hashing::openssl as hash;
    }
    else if #[cfg(feature = "wasm")] {
        pub use signatures::rustcrypto as signature;
        pub(crate) use random::rand as rng;
        pub use hashing::sha2 as hash;
    }
    else {
        // pub use signatures::zcash as signature;
        pub use signatures::dalek as signature;
        pub use random::rand as rng;
        pub use hashing::rustcrypto as hash;
    }
}
