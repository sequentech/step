// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM bindings for the braid mixnet (M3).
//!
//! M3-A unblocks the `--features wasm` build on the v0.6 core. The pre-v0.6
//! bindings — `session.rs`, `verify.rs`, and `board/` — are built on the retired
//! `protocol::{session,board,trustee}` stack (plus `symm` / `TrusteeConfig`'s
//! `encryption_key`, removed in v0.6), so they are **not declared here**. Those
//! files remain on disk, undeclared and uncompiled, as the reference for the
//! M3-B/M3-C port (IndexedDB persistence, browser-fetch transport); a dedicated
//! retirement pass removes them once mined. This mirrors how `protocol.rs` keeps
//! only the dispatch macros while its legacy submodules stay on disk.
//!
//! The real bindings — `SessionTrustee`/`BoardClient` over a browser transport
//! and IndexedDB persistence — land in M3-C.

use wasm_bindgen::prelude::*;

/// Module-load hook: route Rust panics to the browser console. Idempotent.
#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

// Re-export wasm-bindgen-rayon's thread-pool initializer for browser glue: the
// crypto/action layer's parallelism (rayon) runs on a Web Worker pool, which the
// page must start once via `initThreadPool` before heavy compute.
pub use wasm_bindgen_rayon::init_thread_pool as initThreadPool;
