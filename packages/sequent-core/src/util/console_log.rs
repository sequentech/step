// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

extern crate console_error_panic_hook;

#[cfg(feature = "wasm")]
/// Logs formatted output to the browser console when compiled for WASM.
macro_rules! console_log {
    ($($t:tt)*) => {
        ::web_sys::console::log_1(&format_args!($($t)*).to_string().into());
    }
}

#[cfg(not(feature = "wasm"))]
/// Logs formatted output to stdout on native targets.
macro_rules! console_log {
    ($($t:tt)*) => {
        println!("{}", format_args!($($t)*));
    }
}
