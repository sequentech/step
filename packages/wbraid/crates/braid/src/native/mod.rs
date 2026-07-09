// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Native platform-specific modules for the Braid mixnet
//!
//! This module contains functionality that only compiles for native platforms
//! (i.e., not WebAssembly). It requires the `native` feature to be enabled.
//!
// Legacy `board` and `session` modules are retired from the build for M2
// (superseded by crate::board / crate::session); their files remain on disk.
pub mod logging;
pub mod test;
