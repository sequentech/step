// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use anyhow::Result;
use sequent_core::types::tally_sheet_import::TallySheetImportChangeType;
use sequent_core::types::tally_sheets::AreaContestResults;
use tracing::instrument;

use super::hash::hash_area_contest_results;

#[instrument(skip_all, err)]
pub fn classify_change(
    previous: Option<&AreaContestResults>,
    incoming: &AreaContestResults,
) -> Result<TallySheetImportChangeType> {
    let Some(previous) = previous else {
        return Ok(TallySheetImportChangeType::NEW);
    };

    if hash_area_contest_results(previous)? == hash_area_contest_results(incoming)? {
        Ok(TallySheetImportChangeType::UNCHANGED)
    } else {
        Ok(TallySheetImportChangeType::CHANGED)
    }
}

#[instrument(skip_all)]
pub fn render_ballot_box_csv(
    content: &AreaContestResults,
    candidate_names: &HashMap<String, String>,
    candidate_external_ids: &HashMap<String, String>,
) -> String {
    let invalid_votes = content.invalid_votes.clone().unwrap_or_default();
    let mut writer = csv::Writer::from_writer(Vec::new());
    // Every row has four fields and the sink is an in-memory Vec. These writes
    // cannot fail through file I/O; the CSV writer preserves quotes/newlines.
    writer
        .write_record(["field", "candidate_external_id", "candidate_name", "value"])
        .expect("write the fixed review CSV header to memory");
    for (field, value) in [
        ("total_votes", content.total_votes),
        ("total_valid_votes", content.total_valid_votes),
        ("implicit_invalid", invalid_votes.implicit_invalid),
        ("explicit_invalid", invalid_votes.explicit_invalid),
        ("total_blank_votes", content.total_blank_votes),
        ("blank_ballots", content.blank_ballots),
        ("census", content.census),
    ] {
        writer
            .write_record([field, "", "", &value.unwrap_or(0).to_string()])
            .expect("write a fixed-width review CSV row to memory");
    }

    let mut candidate_ids: Vec<_> = content.candidate_results.keys().collect();
    candidate_ids.sort();
    for candidate_id in candidate_ids {
        let votes = content.candidate_results[candidate_id]
            .total_votes
            .unwrap_or(0);
        let external_id = candidate_external_ids
            .get(candidate_id)
            .map(String::as_str)
            .unwrap_or("");
        let name = candidate_names
            .get(candidate_id)
            .map(String::as_str)
            .unwrap_or("");
        writer
            .write_record(["candidate_votes", external_id, name, &votes.to_string()])
            .expect("write a fixed-width candidate CSV row to memory");
    }

    let bytes = writer.into_inner().expect("flush review CSV into memory");
    String::from_utf8(bytes).expect("CSV written from Rust strings is UTF-8")
}
