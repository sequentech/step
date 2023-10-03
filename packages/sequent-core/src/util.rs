// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::base64::Base64Deserialize;
use chrono::{DateTime, Local};
use std::fs;
use strand::context::Ctx;

pub fn get_current_date() -> String {
    let local: DateTime<Local> = Local::now();
    local.format("%-d/%-m/%Y").to_string()
}
