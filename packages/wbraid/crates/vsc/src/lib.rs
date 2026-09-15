// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Cryptography library for the VoteSecure project.
//!
//! A [`Context`](context::Context) instantiates one coherent set of choices —
//! group backend, hashing, RNG, serialization, signatures — that the rest of
//! the crate is generic over; [`RistrettoCtx`](context::RistrettoCtx) is the
//! one braid (this workspace's mixnet trustee crate) uses.
//!
//! ## Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`context`] | ties a group backend + hashing/RNG/serialization/signatures into one [`Context`](context::Context) |
//! | [`traits`] | the group/element/scalar abstractions everything else is generic over |
//! | [`groups`] | curve backends: [`p256`](groups::p256), [`ristretto255`](groups::ristretto255), and generic [`productgroup`](groups::productgroup) |
//! | [`cryptosystem`] | ElGamal and Naor-Yung public-key cryptosystems |
//! | [`dkgd`] | distributed key generation and decryption ([`Dealer`](dkgd::dealer::Dealer) / [`Recipient`](dkgd::recipient::Recipient)) |
//! | [`zkp`] | zero-knowledge proofs: Schnorr, discrete-log equality, PLEQ, and the shuffle proof |
//! | [`utils`] | hashing, RNG, (de)serialization, signatures, error types |
//!
//! See each module's own docs for detail on that module; the overview and a
//! worked example below are this crate's README.

#![allow(dead_code)]
// Only necessary for custom_warning_macro
#![feature(stmt_expr_attributes)]
// Only necessary for custom_warning_macro
#![feature(proc_macro_hygiene)]
#![doc = include_str!("../README.md")]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

/// Defines implementation choices for key cryptographic functionalities.
pub mod context;
pub mod cryptosystem;
#[crate::warning("This module is not optimized.")]
pub mod dkgd;
pub mod groups;
/// Abstractions for curve arithmetic, groups, elements and scalars.
pub mod traits;
/// Utilities such as random number generation, hashing, signatures and serialization.
pub mod utils;
pub mod zkp;

pub use custom_warning_macro::warning;
pub use canonical_derive::Canonical;

/// Create the `cryptography` alias that points to `crate`
///
/// This alias allows applying the `canonical_derive` macro within this crate:
///
/// `canonical_derive` refers to its target traits with `cryptography::`, but
/// _within_ this crate, that reference will not resolve to anything
/// unless we add this alias. Other crates will resolve correctly
/// as they will be importing `cryptography` as a dependency.
#[doc(hidden)]
extern crate self as cryptography;

/// Debug macro that works in both native and WASM contexts.
/// 
/// In WASM builds (when `wasm` feature + `wasm32` target), uses browser `console.log`.
/// In all other cases, uses `info!`.
/// 
/// # Examples
/// ```ignore
/// use strand::debug_log;
/// debug_log!("Processing {} items", count);
/// debug_log!("ZKP verification failed: {:?}", error);
/// ```
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(all(target_arch = "wasm32", feature = "wasm"))]
        {
            use wasm_bindgen::JsValue;
            web_sys::console::log_1(&JsValue::from_str(&format!($($arg)*)));
        }
        #[cfg(not(all(target_arch = "wasm32", feature = "wasm")))]
        {
            info!($($arg)*);
        }
    };
}