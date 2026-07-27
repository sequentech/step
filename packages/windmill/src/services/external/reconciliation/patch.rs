// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Builds the two documents `generate_reconciliation_patches` produces from
//! the computed diff: the downloadable Datafix patch CSV, and the internal
//! Sequent-patch JSON `apply_reconciliation_patch` later applies from. Unlike
//! the diff itself (only the fields that changed), the CSV needs every
//! `DatafixReconciliationField` present per voter regardless of which ones changed,
//! per the "Patch Files Format" spec.

use crate::services::external::datafix_types::{
    DatafixReconciliationField, ParsedDatafixReconciliationRow, FILE_CHANNEL_INTERNET,
};
use crate::services::external::reconciliation::diff::DiffItem;
use crate::services::external::types::{ReconciliationChangeCategory, ReconciliationPatchTarget};
use sequent_core::types::keycloak::ATTR_RESET_VALUE;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use tracing::instrument;

/// Incrementally builds the Datafix patch CSV described in "Patch Files
/// Format" across many batches — the streaming counterpart to serializing
/// the whole diff's items in one call, so a 100k+-row reconciliation never
/// needs the whole diff resident in memory to produce this document. One row
/// per changed voter, every `DatafixReconciliationField` as an `_old`/`_new`
/// pair — unchanged fields repeat this batch's real row value (read off
/// `file_rows_by_username`, this same batch's parsed rows) in both columns,
/// per spec. `NONE` is only used when the voter has no row in the file at
/// all (D, reverse direction — added to Datafix), the spec's own example of
/// a legitimate `NONE`. Unlike the non-streaming version this used to be,
/// voters are written in whatever order their batch encounters them, not
/// sorted across the whole file — each row is independently valid regardless
/// of order, so this only affects the file's own organization, not
/// correctness.
pub struct ExternalPatchCsvWriter<W: Write> {
    writer: W,
    wrote_any_row: bool,
}

impl<W: Write> ExternalPatchCsvWriter<W> {
    #[instrument(skip(writer), err)]
    pub fn start(mut writer: W, sequence: i64, generated_at: i64) -> std::io::Result<Self> {
        writeln!(
            writer,
            "#META,Sequence={sequence},GeneratedAt={generated_at}"
        )?;
        let header: Vec<String> = DatafixReconciliationField::NAMES
            .iter()
            .flat_map(|name| [format!("{name}_old"), format!("{name}_new")])
            .collect();
        writeln!(writer, "VoterID,{}", header.join(","))?;
        Ok(Self {
            writer,
            wrote_any_row: false,
        })
    }

    /// Appends this batch's `target = Datafix` items as patch rows.
    /// `file_rows_by_username` must be this same batch's parsed rows.
    #[instrument(skip_all, err)]
    pub fn write_batch(
        &mut self,
        items: &[DiffItem],
        file_rows_by_username: &HashMap<String, ParsedDatafixReconciliationRow>,
    ) -> std::io::Result<()> {
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

        let mut voters: Vec<&str> = by_voter.keys().copied().collect();
        voters.sort_unstable();

        for voter_username in voters {
            let fields = &by_voter[voter_username];
            let file_row = file_rows_by_username.get(voter_username);
            let values: Vec<String> = DatafixReconciliationField::NAMES
                .iter()
                .flat_map(|name| match fields.get(name) {
                    Some((old_value, new_value)) => {
                        vec![old_value.to_string(), new_value.to_string()]
                    }
                    None => {
                        let value = file_row
                            .and_then(|row| row.field_value(name))
                            .unwrap_or(ATTR_RESET_VALUE);
                        vec![value.to_string(), value.to_string()]
                    }
                })
                .collect();
            writeln!(self.writer, "{voter_username},{}", values.join(","))?;
            self.wrote_any_row = true;
        }
        Ok(())
    }

    /// Returns the underlying writer if any row was ever written, `None`
    /// (matching "nothing to patch") otherwise — the caller decides whether
    /// a Datafix patch document exists at all based on this.
    pub fn finish(self) -> Option<W> {
        self.wrote_any_row.then_some(self.writer)
    }
}

/// Incrementally writes a JSON array of `DiffItem`s across many batches,
/// without ever holding the whole array in memory at once. Reused for both
/// the Sequent-patch document (fed only `target = Sequent`, non-`ROW_FAILURE`
/// items — the *only* thing `apply_reconciliation_patch` reads to decide
/// what to do; it never re-derives this list from the full diff) and the
/// diff envelope's `items` field (fed every item, unfiltered).
pub struct DiffItemArrayWriter<W: Write> {
    writer: W,
    wrote_any: bool,
}

impl<W: Write> DiffItemArrayWriter<W> {
    #[instrument(skip(writer), err)]
    pub fn start(mut writer: W) -> std::io::Result<Self> {
        writer.write_all(b"[")?;
        Ok(Self {
            writer,
            wrote_any: false,
        })
    }

    #[instrument(skip_all, err)]
    pub fn write_batch<'a>(
        &mut self,
        items: impl Iterator<Item = &'a DiffItem>,
    ) -> anyhow::Result<()> {
        for item in items {
            if self.wrote_any {
                self.writer.write_all(b",")?;
            }
            serde_json::to_writer(&mut self.writer, item)?;
            self.wrote_any = true;
        }
        Ok(())
    }

    /// Closes the array and returns the underlying writer.
    pub fn finish(mut self) -> std::io::Result<W> {
        self.writer.write_all(b"]")?;
        Ok(self.writer)
    }
}

/// The `target = Sequent`, non-`ROW_FAILURE` filter `DiffItemArrayWriter`
/// applies when it's building the Sequent-patch document specifically
/// (as opposed to the envelope's `items`, which gets every item unfiltered).
pub fn is_sequent_patch_item(item: &DiffItem) -> bool {
    item.target.is_sequent() && item.category != ReconciliationChangeCategory::ROW_FAILURE
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
    use crate::services::external::types::{
        ReconciliationChangeCategory, SequentReconciliationField,
    };

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
            channel: ATTR_RESET_VALUE.to_string(),
            deleted: "false".to_string(),
        }
    }

    /// Drives `ExternalPatchCsvWriter` over an in-memory buffer for a single
    /// batch, mirroring the removed `build_external_patch_csv`'s own
    /// `Option<String>` shape so the tests below stay close to what they
    /// were checking before this became a streaming writer.
    fn write_csv(
        items: &[DiffItem],
        file_rows_by_username: &HashMap<String, ParsedDatafixReconciliationRow>,
        sequence: i64,
        generated_at: i64,
    ) -> Option<String> {
        let mut writer = ExternalPatchCsvWriter::start(Vec::new(), sequence, generated_at)
            .expect("writing to a Vec<u8> cannot fail");
        writer
            .write_batch(items, file_rows_by_username)
            .expect("writing to a Vec<u8> cannot fail");
        writer
            .finish()
            .map(|buffer| String::from_utf8(buffer).expect("writer only emits valid UTF-8"))
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
        assert!(write_csv(&items, &HashMap::new(), 1, 100).is_none());
    }

    #[test]
    fn includes_the_meta_line_and_header() {
        let items = vec![item(
            "v1",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                ATTR_RESET_VALUE.to_string(),
                FILE_CHANNEL_INTERNET.to_string(),
            )),
        )];
        let csv = write_csv(&items, &HashMap::new(), 5, 1781780700).unwrap();
        let mut lines = csv.lines();
        assert_eq!(
            lines.next(),
            Some("#META,Sequence=5,GeneratedAt=1781780700")
        );
        assert!(lines
            .next()
            .unwrap()
            .starts_with("VoterID,CountyMun_old,CountyMun_new"));
        assert!(csv.contains("v1,"));
    }

    #[test]
    fn unchanged_fields_carry_the_voters_real_row_value_not_none() {
        let items = vec![item(
            "v1",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                ATTR_RESET_VALUE.to_string(),
                FILE_CHANNEL_INTERNET.to_string(),
            )),
        )];
        let file_rows = HashMap::from([("v1".to_string(), file_row("v1"))]);
        let csv = write_csv(&items, &file_rows, 5, 1781780700).unwrap();
        let data_line = csv.lines().nth(2).unwrap();
        assert!(data_line.contains("0014,0014")); // CountyMun_old,CountyMun_new
        assert!(data_line.contains("1990-01-01,1990-01-01")); // DoB_old,DoB_new
        assert!(data_line.contains("false,false")); // Deleted_old,Deleted_new
                                                    // The changed field (Channel) legitimately carries "NONE" as its real
                                                    // old value here — only the placeholder pair for an unchanged field
                                                    // would indicate the bug this test guards against.
        assert!(!data_line.contains("NONE,NONE"));
    }

    #[test]
    fn voter_missing_from_the_file_falls_back_to_none_for_unchanged_fields() {
        let items = vec![item(
            "v2",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Ward(
                ATTR_RESET_VALUE.to_string(),
                "01-P-000".to_string(),
            )),
        )];
        let csv = write_csv(&items, &HashMap::new(), 5, 1781780700).unwrap();
        let data_line = csv.lines().nth(2).unwrap();
        assert!(data_line.contains("NONE,NONE")); // e.g. CountyMun_old,CountyMun_new
    }

    #[test]
    fn external_patch_csv_writer_accumulates_across_batches() {
        let batch_one = vec![item(
            "v1",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                ATTR_RESET_VALUE.to_string(),
                FILE_CHANNEL_INTERNET.to_string(),
            )),
        )];
        let batch_two = vec![item(
            "v2",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Ward(
                ATTR_RESET_VALUE.to_string(),
                "01-P-000".to_string(),
            )),
        )];
        let mut writer = ExternalPatchCsvWriter::start(Vec::new(), 5, 1781780700).unwrap();
        writer.write_batch(&batch_one, &HashMap::new()).unwrap();
        writer.write_batch(&batch_two, &HashMap::new()).unwrap();
        let csv = String::from_utf8(writer.finish().unwrap()).unwrap();
        assert!(csv.contains("v1,"));
        assert!(csv.contains("v2,"));
    }

    #[test]
    fn diff_item_array_writer_produces_a_valid_filtered_json_array() {
        let sequent_item = item(
            "v1",
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::AreaName(
                "01".to_string(),
                "02".to_string(),
            ))),
        );
        let row_failure = DiffItem {
            voter_username: "v2".to_string(),
            target: ReconciliationPatchTarget::Sequent(None),
            category: ReconciliationChangeCategory::ROW_FAILURE,
            failure_reason: Some("bad row".to_string()),
        };
        let datafix_item = item(
            "v3",
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                ATTR_RESET_VALUE.to_string(),
                FILE_CHANNEL_INTERNET.to_string(),
            )),
        );

        // The Sequent-patch document only ever gets `target = Sequent`,
        // non-ROW_FAILURE items — the same filter `is_sequent_patch_item`
        // applies in the real pipeline.
        let mut writer = DiffItemArrayWriter::start(Vec::new()).unwrap();
        writer.write_batch([&sequent_item].into_iter()).unwrap();
        writer
            .write_batch(
                [&row_failure, &datafix_item]
                    .into_iter()
                    .filter(|item| is_sequent_patch_item(item)),
            )
            .unwrap();
        let bytes = writer.finish().unwrap();

        let parsed: Vec<DiffItem> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].voter_username, "v1");
    }

    #[test]
    fn sha256_hex_is_stable() {
        assert_eq!(
            sha256_hex("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
