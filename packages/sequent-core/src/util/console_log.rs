// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

extern crate console_error_panic_hook;

#[cfg(feature = "wasm")]
macro_rules! console_log {
    ($($t:tt)*) => {
        ::web_sys::console::log_1(&format_args!($($t)*).to_string().into());
    }
}

#[cfg(not(feature = "wasm"))]
macro_rules! console_log {
    ($($t:tt)*) => {
        ::tracing::info!("{}", format_args!($($t)*));
    }
}
