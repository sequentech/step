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
pub mod rnd;
/// Serialization frontend. StrandVectors for parallel serialization.
pub mod serialization;
/// Wikstrom proof of shuffle.
pub mod shuffler;
/// Signature frontend.
pub mod signature;
/// Support for threshold ElGamal.
pub mod threshold;
/// Miscellaneous functions.
pub mod util;
#[cfg(feature = "wasm")]
/// Webassembly API.
pub mod wasm;
/// Schnorr and Chaum-Pedersen zero knowledge proofs.
pub mod zkp;
