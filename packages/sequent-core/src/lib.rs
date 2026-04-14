// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![doc = include_str!("lib.docs.md")]
#[macro_use]
extern crate quick_error;
extern crate cfg_if;

/// Ballot structures and helpers.
#[cfg(feature = "default_features")]
pub mod ballot;
/// Ballot style models and selection logic.
#[cfg(feature = "default_features")]
pub mod ballot_style;
/// Shared error types.
#[cfg(feature = "default_features")]
pub mod error;
/// Multi-ballot container types and helpers.
#[cfg(feature = "default_features")]
pub mod multi_ballot;
/// Shared domain types.
pub mod types;
//pub use ballot::*;
/// Ballot encoding and decoding helpers.
#[cfg(feature = "default_features")]
pub mod ballot_codec;
/// Ballot encryption helpers.
#[cfg(feature = "default_features")]
pub mod encrypt;
/// Test fixtures and example data.
#[cfg(feature = "default_features")]
pub mod fixtures;
/// Plaintext ballot interpretation helpers.
#[cfg(feature = "default_features")]
pub mod interpret_plaintext;
/// Mixed-radix encoding and decoding primitives.
#[cfg(feature = "default_features")]
pub mod mixed_radix;
/// Plaintext ballot models and validation helpers.
#[cfg(feature = "default_features")]
pub mod plaintext;

/// `WIT` bindings for plugin integration.
#[cfg(feature = "plugins_wit")]
pub mod plugins_wit;

/// Shared serialization helpers.
#[cfg(feature = "default_features")]
pub mod serialization;
/// Domain services and business logic.
#[cfg(feature = "default_features")]
pub mod services;
/// `SQLite`-backed helpers.
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// General-purpose utilities.
#[cfg(feature = "default_features")]
pub mod util;

/// Temporary file and path helpers for reports.
#[cfg(all(feature = "reports", feature = "default_features"))]
pub mod temp_path;

/// Signature helpers and verification utilities.
#[cfg(all(feature = "signatures", feature = "default_features"))]
pub mod signatures;

/// `WebAssembly` bindings exported to frontend packages.
#[cfg(all(feature = "wasm", feature = "default_features"))]
pub mod wasm;
