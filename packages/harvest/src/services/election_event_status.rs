// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use immu_board::Board;
use rocket::serde::{Deserialize, Serialize};
use serde_json::value::Value;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct ElectionEventStatus {
    pub config_created: Option<bool>,
}

pub fn is_config_created(status_opt: Option<ElectionEventStatus>) -> bool {
    match status_opt {
        None => false,
        Some(status) => status.config_created.unwrap_or(false),
    }
}
