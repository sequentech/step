// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod api_types;

// Native-only modules
#[cfg(feature = "native")]
pub mod db;
#[cfg(feature = "native")]
pub mod handlers;
#[cfg(feature = "native")]
pub mod s3;
#[cfg(feature = "native")]
pub mod state;

/// Seconds elapsed since `std::time::UNIX_EPOCH`.
pub type Timestamp = u64;
#[cfg(feature = "native")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "native")]
pub fn timestamp() -> Timestamp {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}

#[cfg(target_arch = "wasm32")]
pub fn timestamp() -> Timestamp {
    // Use JavaScript Date.now() for WASM (returns milliseconds since epoch)
    (js_sys::Date::now() / 1000.0) as u64
}

pub fn get_schema_version() -> String {
    "1".to_string()
}
