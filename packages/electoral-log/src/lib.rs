// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[cfg(feature = "native")]
pub mod adapters;
pub mod domain;
pub mod messages;
#[cfg(feature = "native")]
pub mod ports;
#[cfg(feature = "native")]
pub mod proofs;
pub mod seal;
#[cfg(feature = "native")]
pub mod service;
#[cfg(feature = "native")]
pub mod util;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::messages::newtypes::Timestamp;
pub use domain::*;
#[cfg(feature = "native")]
pub use service::BoardClient;

pub fn get_schema_version() -> String {
    "1".to_string()
}

/// Returns the minimum reader schema version required for a row containing the
/// append-only `CastVoteWithChannel` body. The stored version is compatibility
/// metadata for readers; it does not change the encoding of other statements.
pub fn get_cast_vote_channel_schema_version() -> String {
    "2".to_string()
}

pub fn timestamp() -> Timestamp {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}
