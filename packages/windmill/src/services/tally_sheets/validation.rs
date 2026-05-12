// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tally sheet validation helpers.
use crate::types::error::Result;
use anyhow::anyhow;
use sequent_core::ballot::{Candidate, Contest};
use sequent_core::types::hasura::core::TallySheet;
use sequent_core::types::tally_sheets::InvalidVotes;
use std::collections::HashMap;
use tracing::instrument;

#[instrument(skip_all, err)]
/// Validates that a stored tally sheet is internally consistent for a contest.
///
/// This performs basic accounting checks (votes vs. census, invalid/valid totals)
/// and ensures each candidate result refers to a candidate that exists in the
/// contest definition.
///
/// # Errors
///
/// Returns an error if the tally sheet is missing content, has inconsistent vote
/// totals, or contains candidate results that don't match the contest.
///
/// # Panics
///
/// Panics if vote totals overflow `u64` while being summed for validation.
pub fn validate_tally_sheet(tally_sheet: &TallySheet, contest: &Contest) -> Result<()> {
    let Some(results) = tally_sheet.content.clone() else {
        return Err(anyhow!("Invalid tally sheet {tally_sheet:?}, content missing").into());
    };
    if results.total_votes > results.census {
        return Err(
            anyhow!("Invalid tally sheet {tally_sheet:?}, total_votes higher than census").into(),
        );
    }
    let invalid_votes = results.invalid_votes.unwrap_or(InvalidVotes::default());
    let total_invalid_votes_calculated = invalid_votes
        .explicit_invalid
        .unwrap_or(0)
        .checked_add(invalid_votes.implicit_invalid.unwrap_or(0))
        .expect("total invalid votes overflow");
    let total_invalid_votes = invalid_votes.total_invalid.unwrap_or(0);
    if total_invalid_votes != total_invalid_votes_calculated {
        return Err(anyhow!(
            "Invalid tally sheet {tally_sheet:?}, inconsistent total invalid votes"
        )
        .into());
    }
    let total_votes = results.total_votes.unwrap_or(0);
    let total_valid_votes = results.total_valid_votes.unwrap_or(0);
    let total_blank_votes = results.total_blank_votes.unwrap_or(0);
    let votes_accounted = total_invalid_votes
        .checked_add(total_valid_votes)
        .expect("vote totals overflow");
    if votes_accounted != total_votes {
        return Err(
            anyhow!("Invalid tally sheet {tally_sheet:?}, inconsistent total votes",).into(),
        );
    }
    let total_valid_votes_calc: u64 = results
        .candidate_results
        .values()
        .map(|candidate_result| -> u64 { candidate_result.total_votes.unwrap_or(0) })
        .sum();

    /*if total_valid_votes != total_valid_votes_calc + total_blank_votes {
        return Err(anyhow!(
            "Invalid tally sheet {:?}, inconsistent total valid votes",
            tally_sheet
        )
        .into());
    }*/
    let candidates_map: HashMap<String, Candidate> = contest
        .candidates
        .clone()
        .into_iter()
        .map(|candidate| (candidate.id.clone(), candidate.clone()))
        .collect();
    for (candidate_id, candidate_data) in &results.candidate_results {
        if *candidate_id != candidate_data.candidate_id {
            return Err(anyhow!(
                "Invalid tally sheet {tally_sheet:?}, inconsistent candidate result {candidate_data:?}, {candidate_id}",
            ).into());
        }
        if !candidates_map.contains_key(&candidate_data.candidate_id) {
            return Err(anyhow!(
                "Invalid tally sheet {tally_sheet:?}, can't find candidate {candidate_data:?}"
            )
            .into());
        }
    }
    Ok(())
}
