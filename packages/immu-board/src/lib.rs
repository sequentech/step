// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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

pub use board_client::*;

pub mod board_client;
pub mod util;
