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
pub mod signature;
pub mod signer;
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

thread_local! {
    /// The time every statement made during a [`signer::batch`] carries.
    static BATCH_TIMESTAMP: std::cell::Cell<Option<Timestamp>> = const { std::cell::Cell::new(None) };
}

/// Runs `f` with [`timestamp`] fixed to the current second on this thread
/// (as a batch's two passes need the same bytes), restoring it afterwards.
pub(crate) fn with_fixed_timestamp<T>(f: impl FnOnce() -> T) -> T {
    with_timestamp(timestamp(), f)
}

/// Runs `f` with [`timestamp`] fixed to `at` on this thread, restoring it
/// afterwards: the statements `f` makes carry that time. A statement queued
/// unsigned is made when it is signed, with the time it happened.
pub fn with_timestamp<T>(at: Timestamp, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<Timestamp>);
    impl Drop for Restore {
        fn drop(&mut self) {
            BATCH_TIMESTAMP.with(|cell| cell.set(self.0));
        }
    }
    let _restore = Restore(BATCH_TIMESTAMP.with(|cell| cell.replace(Some(at))));
    f()
}

pub fn timestamp() -> Timestamp {
    if let Some(fixed) = BATCH_TIMESTAMP.with(|cell| cell.get()) {
        return fixed;
    }
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}
