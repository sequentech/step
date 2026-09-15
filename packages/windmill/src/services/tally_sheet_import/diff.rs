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
    let mut lines = vec!["field,candidate_external_id,candidate_name,value".to_string()];
    lines.push(format!(
        "total_votes,,,{value}",
        value = content.total_votes.unwrap_or(0)
    ));
    lines.push(format!(
        "total_valid_votes,,,{value}",
        value = content.total_valid_votes.unwrap_or(0)
    ));
    lines.push(format!(
        "implicit_invalid,,,{value}",
        value = invalid_votes.implicit_invalid.unwrap_or(0)
    ));
    lines.push(format!(
        "explicit_invalid,,,{value}",
        value = invalid_votes.explicit_invalid.unwrap_or(0)
    ));
    lines.push(format!(
        "total_blank_votes,,,{value}",
        value = content.total_blank_votes.unwrap_or(0)
    ));
    lines.push(format!(
        "blank_ballots,,,{value}",
        value = content.blank_ballots.unwrap_or(0)
    ));
    lines.push(format!(
        "census,,,{value}",
        value = content.census.unwrap_or(0)
    ));

    let mut candidate_ids = content
        .candidate_results
        .keys()
        .cloned()
        .collect::<Vec<String>>();
    candidate_ids.sort();

    for candidate_id in candidate_ids {
        let votes = content
            .candidate_results
            .get(&candidate_id)
            .and_then(|candidate| candidate.total_votes)
            .unwrap_or(0);
        let candidate_external_id = candidate_external_ids
            .get(&candidate_id)
            .cloned()
            .unwrap_or_default()
            .replace(',', " ");
        let candidate_name = candidate_names
            .get(&candidate_id)
            .cloned()
            .unwrap_or_default()
            .replace(',', " ");
        lines.push(format!(
            "candidate_votes,{candidate_external_id},{candidate_name},{votes}"
        ));
    }

    format!("{}\n", lines.join("\n"))
}
