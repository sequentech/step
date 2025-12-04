// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM bindings for the Braid mixnet protocol
//!
//! This module provides WebAssembly bindings for running Braid trustees in a browser.
//! It requires the `wasm` feature to be enabled.

pub mod session;
pub mod board;

pub use session::WasmSession;

// Re-export wasm-bindgen-rayon's initThreadPool for browser usage
// This provides parallel computation support via Web Workers
pub use wasm_bindgen_rayon::init_thread_pool as initThreadPool;
