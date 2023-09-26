// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use chrono::{DateTime, Local};
use std::fs;

pub fn read_ballot_fixture() -> AuditableBallot {
    let contents = fs::read_to_string("fixtures/ballot.json")
        .expect("Something went wrong reading the file");
    serde_json::from_str(&contents).unwrap()
}

pub fn get_current_date() -> String {
    let local: DateTime<Local> = Local::now();
    local.format("%-d/%-m/%Y").to_string()
}
