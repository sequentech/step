// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use regex::Regex;
use std::collections::HashMap;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(input))]
pub fn replace_uuids(input: &str, keep: Vec<String>) -> String {
    let uuid_regex =
        Regex::new(r"\b[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}\b")
            .unwrap();

    let mut seen_uuids = HashMap::new();

    uuid_regex
        .replace_all(input, |caps: &regex::Captures| {
            let old_uuid = caps.get(0).unwrap().as_str();
            seen_uuids
                .entry(old_uuid.to_owned())
                .or_insert_with(|| Uuid::new_v4().to_string())
                .clone()
        })
        .into_owned()
}
