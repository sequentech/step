// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Headless-browser test that the full v0.6 protocol runs correctly compiled to
//! wasm32 (M3-C).
//!
//! Runs DKG → encrypt → mix → threshold-decrypt over an in-memory board (no b4)
//! and asserts the decrypted plaintexts match the inputs. The protocol logic is
//! covered natively by `protocol_test_memory`; this guards against wasm-specific
//! regressions (arithmetic, serialization, RNG, the ascent datalog under wasm).
//!
//! Run via `test-wasm.ps1` (headless Chrome). Compiled only for `wasm32` under
//! the `wasm-core` feature, so native `cargo test` skips it.

#![cfg(all(target_arch = "wasm32", feature = "wasm-core"))]

use wasm_bindgen_test::*;

use braid::wasm::emulator::run_in_memory;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn protocol_runs_in_memory() {
    // Small params keep the single-threaded wasm crypto quick.
    let result = run_in_memory(3, 2, 20, 2).await.expect("protocol run");
    assert!(
        result.success,
        "decrypted plaintexts should match the encrypted inputs"
    );
    assert_eq!(result.trustees, 3);
    assert_eq!(result.threshold, 2);
    assert_eq!(result.ciphertexts, 20);
}
