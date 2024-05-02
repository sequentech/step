// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strand::backend::ristretto::RistrettoCtx;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn test_protocol_wasm(ciphertexts: u32, batches: usize) {
    tracing_wasm::set_as_global_default();
    crate::util::init_log(false);
    crate::test::protocol_test::run(ciphertexts, batches, RistrettoCtx);
}

pub use wasm_bindgen_rayon::init_thread_pool;
