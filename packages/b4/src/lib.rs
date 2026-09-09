// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod api_types;
pub mod messages;

// Native-only modules
#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "native")]
pub mod db;
#[cfg(feature = "native")]
pub mod handlers;
#[cfg(feature = "native")]
pub mod s3;
#[cfg(feature = "native")]
pub mod state;

use crate::messages::newtypes::Timestamp;
#[cfg(feature = "client")]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(feature = "client")]
pub(crate) fn system_time_from_timestamp(seconds: Timestamp) -> Option<SystemTime> {
    let duration = Duration::from_secs(seconds);
    UNIX_EPOCH.checked_add(duration)
}

#[cfg(feature = "client")]
pub(crate) fn timestamp_from_system_time(system_time: &SystemTime) -> Timestamp {
    let since_the_epoch = system_time
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}

pub fn get_schema_version() -> String {
    "1".to_string()
}

// Re-export HTTP message types for convenience
pub use messages::http_message::{HttpB3Message, HttpBoardMessages};
