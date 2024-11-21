// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod messages;

#[cfg(feature = "client")]
pub mod client;
pub mod grpc;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::messages::newtypes::Timestamp;

pub fn timestamp() -> Timestamp {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}

pub(crate) fn system_time_from_timestamp(seconds: Timestamp) -> Option<SystemTime> {
    let duration = Duration::from_secs(seconds);
    UNIX_EPOCH.checked_add(duration)
}

pub(crate) fn timestamp_from_system_time(system_time: &SystemTime) -> Timestamp {
    let since_the_epoch = system_time
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}

pub fn get_schema_version() -> String {
    "1".to_string()
}
