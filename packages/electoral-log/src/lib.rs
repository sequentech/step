// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
pub mod client;
pub mod messages;
pub mod util;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::messages::newtypes::Timestamp;
pub use client::board_client::*;

pub fn get_schema_version() -> String {
    "1".to_string()
}

pub fn timestamp() -> Timestamp {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}

#[macro_export]
macro_rules! assign_value {
    ($enum_variant:path, $value:expr, $target:ident) => {
        match $value.value.as_ref() {
            Some($enum_variant(inner)) => {
                $target = inner.clone();
            }
            _ => {
                return Err(anyhow!(
                    r#"invalid column value for `$enum_variant`, `$value`, `$target`"#
                ));
            }
        }
    };
}
