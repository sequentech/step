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
/// Support for distributed Elgamal.
#[allow(dead_code)]
mod keymaker;
/// Random number generation frontend.
#[doc(hidden)]
pub mod rnd;
/// Serialization frontend. StrandVectors for parallel serialization.
#[doc(hidden)]
pub mod serialization;
/// Wikstrom proof of shuffle.
#[doc(hidden)]
pub mod shuffler;
/// Signature frontend (Zcash).
pub mod signature;
/// Signature frontend (Dalek).
pub mod signature2;
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
