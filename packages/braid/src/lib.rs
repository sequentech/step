// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
extern crate cfg_if;

pub mod protocol;
pub mod util;

// Platform-specific modules
// Note: native::board is available in both builds because NoOpStorage is used by WASM temporarily
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod native;
#[cfg(feature = "wasm")]
pub mod wasm;
