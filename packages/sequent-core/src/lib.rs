// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared core library for the Sequent voting platform.
//!
//! This crate holds ballot structures, encryption helpers, serialization, and
//! domain types used across backend services and browser-facing WASM builds
//! (voting portal, ballot verifier, admin tooling). Feature flags gate
//! optional integrations such as Keycloak, `SQLite`, PDF reports, and plugin
//! runtimes.

#[macro_use]
extern crate quick_error;
extern crate cfg_if;

/// Auditable and hashable ballot representations, election configuration, and
/// voter-facing presentation types.
#[cfg(feature = "default_features")]
pub mod ballot;
/// Ballot style definitions that scope which contests a voter may access.
#[cfg(feature = "default_features")]
pub mod ballot_style;
/// Ballot processing and validation error types.
#[cfg(feature = "default_features")]
pub mod error;
/// Multi-contest ballot structures and operations.
#[cfg(feature = "default_features")]
pub mod multi_ballot;
/// Shared domain types for ceremonies, permissions, results, and integrations.
pub mod types;
//pub use ballot::*;
/// Encoding and decoding of ballot plaintext into auditable ciphertexts.
#[cfg(feature = "default_features")]
pub mod ballot_codec;
/// ElGamal encryption of decoded votes and ballot hashing.
#[cfg(feature = "default_features")]
pub mod encrypt;
/// Test and sample data generators for ballots and elections.
#[cfg(feature = "default_features")]
pub mod fixtures;
/// Interpretation of decoded plaintext votes against contest rules.
#[cfg(feature = "default_features")]
pub mod interpret_plaintext;
/// Mixed-radix numeral conversions used by the ballot codec.
#[cfg(feature = "default_features")]
pub mod mixed_radix;
/// Decoded vote choice and contest structures before encryption.
#[cfg(feature = "default_features")]
pub mod plaintext;

/// WASM plugin interface types and runtime helpers.
#[cfg(feature = "plugins_wit")]
pub mod plugins_wit;

/// Serialization helpers shared across ballot and API types.
#[cfg(feature = "default_features")]
pub mod serialization;
/// External service clients (Keycloak, S3, reports, authorization, etc.).
#[cfg(feature = "default_features")]
pub mod services;
/// SQLite schema access for offline or embedded election data.
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// General-purpose utilities (locale, dates, paths, logging, etc.).
#[cfg(feature = "default_features")]
pub mod util;

/// Temporary file path helpers for report generation.
#[cfg(all(feature = "reports", feature = "default_features"))]
pub mod temp_path;

/// Cryptographic signature helpers for trustee and shell workflows.
#[cfg(all(feature = "signatures", feature = "default_features"))]
pub mod signatures;

/// Webassembly API.
#[cfg(all(feature = "wasm", feature = "default_features"))]
pub mod wasm;
