// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(input))]
pub fn replace_uuids(input: &str, keep: Vec<String>) -> String {
    let uuid_regex =
        Regex::new(r"[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}")
            .unwrap();

    let mut seen_uuids = HashMap::new();
    let keep_set: HashSet<String> = keep.into_iter().collect();

    uuid_regex
        .replace_all(input, |caps: &regex::Captures| {
            let old_uuid = caps.get(0).unwrap().as_str().to_string();
            if keep_set.contains(&old_uuid) {
                old_uuid.clone()
            } else {
                seen_uuids
                    .entry(old_uuid.clone())
                    .or_insert_with(|| Uuid::new_v4().to_string())
                    .clone()
            }
        })
        .into_owned()
}
