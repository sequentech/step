// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Builds the two documents `generate_reconciliation_patches` produces from
//! the computed diff: the downloadable Datafix patch CSV, and the internal
//! Sequent-patch JSON `apply_reconciliation_patch` later applies from. Unlike
//! the diff itself (only the fields that changed), the CSV needs every
//! `DatafixReconciliationField` present per voter regardless of which ones changed,
//! per the "Patch Files Format" spec.

use crate::services::external::reconciliation::diff::DiffItem;
use crate::services::external::datafix_types::{
    DatafixReconciliationField, ParsedDatafixReconciliationRow,
};
use crate::services::external::types::{ReconciliationChangeCategory, ReconciliationPatchTarget};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::instrument;

/// Serializes the `target = Datafix` items into the patch CSV described in
/// "Patch Files Format": a `#META` line carrying the source file's `Sequence`
/// and this patch's own `GeneratedAt`, then one row per changed voter with
/// every `DatafixReconciliationField` as an `_old`/`_new` pair — unchanged
/// fields repeat this voter's real value (read off `file_rows_by_username`,
/// the same parsed rows the diff was computed from) in both columns, per
/// spec. `NONE` is only used when the voter has no row in the file at all
/// (D, reverse direction — added to Datafix), the spec's own example of a
/// legitimate `NONE`. Returns `None` if there is nothing to patch (no
/// `target = Datafix` items at all).
#[instrument(skip(items, file_rows_by_username))]
pub fn build_external_patch_csv(
    items: &[DiffItem],
    file_rows_by_username: &HashMap<String, ParsedDatafixReconciliationRow>,
    sequence: i64,
    generated_at: i64,
) -> Option<String> {
    let mut by_voter: HashMap<&str, HashMap<&'static str, (&str, &str)>> = HashMap::new();
    for item in items {
        let Some(field) = item.target.datafix_field() else {
            continue;
        };
        by_voter
            .entry(item.voter_username.as_str())
            .or_default()
            .insert(field.name(), field.old_new());
    }

    if by_voter.is_empty() {
        return None;
    }

    let mut lines = vec![format!("#META,Sequence={sequence},GeneratedAt={generated_at}")];
    let header: Vec<String> = DatafixReconciliationField::NAMES
        .iter()
        .flat_map(|name| [format!("{name}_old"), format!("{name}_new")])
        .collect();
    lines.push(format!("VoterID,{}", header.join(",")));

    let mut voters: Vec<&str> = by_voter.keys().copied().collect();
    voters.sort_unstable();

    for voter_username in voters {
        let fields = &by_voter[voter_username];
        let file_row = file_rows_by_username.get(voter_username);
        let values: Vec<String> = DatafixReconciliationField::NAMES
            .iter()
            .flat_map(|name| match fields.get(name) {
                Some((old_value, new_value)) => vec![old_value.to_string(), new_value.to_string()],
                None => {
                    let value = file_row.and_then(|row| row.field_value(name)).unwrap_or("NONE");
                    vec![value.to_string(), value.to_string()]
                }
            })
            .collect();
        lines.push(format!("{voter_username},{}", values.join(",")));
    }

    Some(lines.join("\n"))
}

/// Serializes the `target = Sequent`, non-`ROW_FAILURE` items into the JSON
/// document `apply_reconciliation_patch` fetches and applies from — this is
/// the *only* thing apply reads to decide what to do; it never re-derives
/// this list from the full diff.
#[instrument(skip(items))]
pub fn build_sequent_patch_json(items: &[DiffItem]) -> Result<Vec<u8>> {
    let sequent_items: Vec<&DiffItem> = items
        .iter()
        .filter(|item| {
            item.target.is_sequent() && item.category != ReconciliationChangeCategory::ROW_FAILURE
        })
        .collect();
    Ok(serde_json::to_vec(&sequent_items)?)
}

/// Serializes row failures (both diff-time `ROW_FAILURE` items and any
/// apply-time failures collected while applying) into the downloadable "row
/// failures report" CSV.
#[instrument(skip(rows))]
pub fn build_row_failures_csv(rows: &[(String, String)]) -> String {
    let mut lines = vec!["VoterID,Reason".to_string()];
    for (voter_username, reason) in rows {
        lines.push(format!("{voter_username},\"{}\"", reason.replace('"', "'")));
    }
    lines.join("\n")
}

#[instrument(skip_all)]
pub fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::external::types::{ReconciliationChangeCategory, SequentReconciliationField};

    fn item(voter: &str, target: ReconciliationPatchTarget) -> DiffItem {
        DiffItem {
            voter_username: voter.to_string(),
            target,
            category: ReconciliationChangeCategory::PROFILE_UPDATE,
            failure_reason: None,
        }
    }

    fn file_row(voter_id: &str) -> ParsedDatafixReconciliationRow {
        ParsedDatafixReconciliationRow {
            county_mun: "0014".to_string(),
            voter_id: voter_id.to_string(),
            dob: "1990-01-01".to_string(),
            ward: "01".to_string(),
            poll: "000".to_string(),
            school_support_code: "P".to_string(),
            channel: "NONE".to_string(),
            deleted: "false".to_string(),
        }
    }

    #[test]
    fn returns_none_when_there_is_nothing_for_the_external_system() {
        let items = vec![item(
            "v1",
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::AreaName(
                "01".to_string(),
                "02".to_string(),
            ))),
        )];
        assert!(build_external_patch_csv(&items, &HashMap::new(), 1, 100).is_none());
    }

    #[test]
    fn includes_the_meta_line_and_header() {
        let items = vec![item(
            "v1",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                "NONE".to_string(),
                "INTERNET".to_string(),
            )),
        )];
        let csv = build_external_patch_csv(&items, &HashMap::new(), 5, 1781780700).unwrap();
        let mut lines = csv.lines();
        assert_eq!(lines.next(), Some("#META,Sequence=5,GeneratedAt=1781780700"));
        assert!(lines.next().unwrap().starts_with("VoterID,CountyMun_old,CountyMun_new"));
        assert!(csv.contains("v1,"));
    }

    #[test]
    fn unchanged_fields_carry_the_voters_real_row_value_not_none() {
        let items = vec![item(
            "v1",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                "NONE".to_string(),
                "INTERNET".to_string(),
            )),
        )];
        let file_rows = HashMap::from([("v1".to_string(), file_row("v1"))]);
        let csv = build_external_patch_csv(&items, &file_rows, 5, 1781780700).unwrap();
        let data_line = csv.lines().nth(2).unwrap();
        assert!(data_line.contains("0014,0014")); // CountyMun_old,CountyMun_new
        assert!(data_line.contains("1990-01-01,1990-01-01")); // DoB_old,DoB_new
        assert!(data_line.contains("false,false")); // Deleted_old,Deleted_new
        assert!(!data_line.contains("NONE"));
    }

    #[test]
    fn voter_missing_from_the_file_falls_back_to_none_for_unchanged_fields() {
        let items = vec![item(
            "v2",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Ward(
                "NONE".to_string(),
                "01-P-000".to_string(),
            )),
        )];
        let csv = build_external_patch_csv(&items, &HashMap::new(), 5, 1781780700).unwrap();
        let data_line = csv.lines().nth(2).unwrap();
        assert!(data_line.contains("NONE,NONE")); // e.g. CountyMun_old,CountyMun_new
    }

    #[test]
    fn sha256_hex_is_stable() {
        assert_eq!(
            sha256_hex("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
