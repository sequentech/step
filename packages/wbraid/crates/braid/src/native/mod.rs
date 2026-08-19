// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Native platform-specific modules: HTTP+S3 transport, SQLite persistence,
//! logging, and the native test harnesses.

pub mod http_transport;
pub mod logging;
pub mod persistence;
pub mod test;

use crate::messages::newtypes::Timestamp;

/// The current wall-clock time as a [`Timestamp`] (seconds since Unix epoch).
///
/// Used to stamp the informational `date` field on outgoing protocol messages
/// (§10.2). The timestamp plays no part in verification or the datalog.
pub fn timestamp() -> Timestamp {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs()
}
