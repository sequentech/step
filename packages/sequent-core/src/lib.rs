// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![feature(int_roundings)]
#[macro_use]
extern crate quick_error;
extern crate cfg_if;

pub mod ballot;
pub mod ballot_style;
pub mod error;
pub mod multi_ballot;
pub mod types;
//pub use ballot::*;
pub mod ballot_codec;
pub mod encrypt;
pub mod fixtures;
pub mod interpret_plaintext;
pub mod mixed_radix;
pub mod plaintext;
pub mod serialization;
pub mod services;
pub mod util;

#[cfg(feature = "reports")]
pub mod temp_path;

#[cfg(feature = "signatures")]
pub mod signatures;

/// Webassembly API.
#[cfg(feature = "wasm")]
pub mod wasm;
