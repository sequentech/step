// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(input))]
/// Replaces UUIDs in the input string, except those in the keep list or fixed config.
///
/// # Panics
/// Panics if the UUID regex is invalid or if a capture group is missing (should not happen).
#[allow(clippy::unwrap_used)] // Safe: regex is a static, known-valid constant; failure would indicate a programming error.
pub fn replace_uuids(
    input: &str,
    keep: &[String],
) -> (String, HashMap<String, String>) {
    let uuid_regex =
        Regex::new(r"[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}")
            .expect("Invalid UUID regex");
    let fixed_uuids_from_config = env::var("ELECTION_EVENT_FIXED_UUIDS")
        .unwrap_or_default()
        .split(',')
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>();
    let mut keep_all = keep.to_vec();
    keep_all.extend(fixed_uuids_from_config);

    let mut seen_uuids = HashMap::new();
    let keep_set: HashSet<String> = keep_all.into_iter().collect();

    let result = uuid_regex
        .replace_all(input, |caps: &regex::Captures| {
            let old_uuid = caps
                .get(0)
                .expect("capture group 0 should always be present")
                .as_str()
                .to_string();
            if keep_set.contains(&old_uuid) {
                old_uuid.clone()
            } else {
                seen_uuids
                    .entry(old_uuid.clone())
                    .or_insert_with(|| Uuid::new_v4().to_string())
                    .clone()
            }
        })
        .into_owned();
    (result, seen_uuids)
}
