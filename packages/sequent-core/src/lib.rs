// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![feature(int_roundings)]
#[macro_use]
extern crate quick_error;
extern crate cfg_if;

#[cfg(feature = "default_features")]
pub mod auditable_ballot;
pub mod ballot;
#[cfg(feature = "default_features")]
pub mod ballot_style;
#[cfg(feature = "default_features")]
pub mod error;
#[cfg(feature = "default_features")]
pub mod multi_ballot;
pub mod types;
//pub use ballot::*;
#[cfg(feature = "default_features")]
pub mod ballot_codec;
#[cfg(feature = "default_features")]
pub mod encrypt;
#[cfg(feature = "default_features")]
pub mod fixtures;
#[cfg(feature = "default_features")]
pub mod interpret_plaintext;
#[cfg(feature = "default_features")]
pub mod mixed_radix;
#[cfg(feature = "default_features")]
pub mod plaintext;

#[cfg(feature = "plugins_wit")]
pub mod plugins_wit;

pub mod serialization;
#[cfg(feature = "default_features")]
pub mod services;

pub mod util;

#[cfg(all(feature = "reports", feature = "default_features"))]
pub mod temp_path;

#[cfg(all(feature = "signatures", feature = "default_features"))]
pub mod signatures;

/// Webassembly API.
#[cfg(all(feature = "wasm", feature = "default_features"))]
pub mod wasm;

pub mod plugins;
pub mod std_temp_path;
