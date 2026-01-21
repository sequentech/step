// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[cfg(feature = "wasmtest")]
pub mod bench;
#[cfg(all(feature = "wasmtest", feature = "wasmrayon"))]
pub mod demo;
#[cfg(feature = "wasmtest")]
pub mod test;

#[cfg(feature = "wasmrayon")]
pub use wasm_bindgen_rayon::init_thread_pool;
