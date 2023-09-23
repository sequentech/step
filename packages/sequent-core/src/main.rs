// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
mod ballot;
mod hasura_types;
use crate::ballot::AuditableBallot;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(AuditableBallot);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
