// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Native platform-specific modules for the Braid mixnet
//!
//! This module contains functionality that only compiles for native platforms
//! (i.e., not WebAssembly). It requires the `native` feature to be enabled.
//!
//! Note: The `board` module is also available in WASM builds because NoOpStorage
//! is temporarily used by WASM until proper browser-based storage is implemented.
//! The `verify` module is also available for WASM verifier support.

// Available in both native and WASM
pub mod board;
pub mod verify;

// Native-only modules
#[cfg(feature = "native")]
pub mod logging;
#[cfg(feature = "native")]
pub mod session;
#[cfg(feature = "native")]
pub mod test;
