// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM bindings for the braid mixnet (M3).
//!
//! The bindings are `SessionTrustee`/`BoardClient` over a browser-`fetch`
//! transport with IndexedDB persistence, exposed through the interactive
//! [`emulator`]. (The pre-v0.6 wasm bindings were archived to `legacy/`.)
//!
//! This module is gated on `wasm-core` (the base wasm build). The Web Worker
//! thread pool (`initThreadPool`) is only present under the full `wasm` feature,
//! which adds `wasm-bindgen-rayon`; `wasm-core` builds (e.g. tests) omit it so
//! they need no atomics / SharedArrayBuffer.

pub mod emulator;

use wasm_bindgen::prelude::*;

/// Module-load hook: route Rust panics to the browser console. Idempotent.
#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

// Re-export wasm-bindgen-rayon's thread-pool initializer for browser glue: the
// crypto/action layer's parallelism (rayon) runs on a Web Worker pool, which the
// page must start once via `initThreadPool` before heavy compute.
#[cfg(feature = "wasm")]
pub use wasm_bindgen_rayon::init_thread_pool as initThreadPool;
