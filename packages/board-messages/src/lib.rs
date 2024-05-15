// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod braid;
pub mod electoral_log;

use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn timestamp() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}

pub fn getSchemaVersion() -> String {
    "1".to_string()
}
