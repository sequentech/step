// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use anyhow::anyhow;
use sequent_core::ballot::{Candidate, Contest};
use sequent_core::types::hasura::core::TallySheet;
use std::collections::HashMap;
use tracing::instrument;

#[instrument(skip_all, err)]
pub fn validate_tally_sheet(tally_sheet: &TallySheet, contest: &Contest) -> Result<()> {
    let tally_sheet_ref = format!(
        "tally sheet {} (area {}, contest {})",
        tally_sheet.id, tally_sheet.area_id, tally_sheet.contest_id
    );
    let Some(content) = tally_sheet.content.clone() else {
        return Err(anyhow!("Invalid {tally_sheet_ref}: content missing").into());
    };
    if content.total_votes > content.census {
        return Err(anyhow!(
            "Invalid {tally_sheet_ref}: total_votes ({:?}) higher than census ({:?})",
            content.total_votes,
            content.census
        )
        .into());
    }
    let invalid_votes = content.invalid_votes.unwrap_or(Default::default());
    let total_invalid_votes_calculated =
        invalid_votes.explicit_invalid.unwrap_or(0) + invalid_votes.implicit_invalid.unwrap_or(0);
    let total_invalid_votes = invalid_votes.total_invalid.unwrap_or(0);
    if total_invalid_votes != total_invalid_votes_calculated {
        return Err(anyhow!(
            "Invalid {tally_sheet_ref}: inconsistent total invalid votes ({total_invalid_votes} != {total_invalid_votes_calculated})"
        )
        .into());
    }
    let total_votes = content.total_votes.unwrap_or(0);
    let total_valid_votes = content.total_valid_votes.unwrap_or(0);
    let total_blank_votes = content.total_blank_votes.unwrap_or(0);
    if total_invalid_votes + total_valid_votes != total_votes {
        return Err(anyhow!(
            "Invalid {tally_sheet_ref}: inconsistent total votes ({total_invalid_votes} + {total_valid_votes} != {total_votes})"
        )
        .into());
    }
    let total_valid_votes_calc: u64 = content
        .candidate_results
        .values()
        .map(|candidate_result| -> u64 { candidate_result.total_votes.clone().unwrap_or(0) })
        .sum();

    if total_valid_votes != total_valid_votes_calc + total_blank_votes {
        return Err(anyhow!(
            "Invalid {tally_sheet_ref}: inconsistent total valid votes ({total_valid_votes} != {total_valid_votes_calc} + {total_blank_votes})"
        )
        .into());
    }

    let candidates_map: HashMap<String, Candidate> = contest
        .candidates
        .clone()
        .into_iter()
        .map(|candidate| (candidate.id.clone(), candidate.clone()))
        .collect();

    for (candidate_id, candidate_data) in content.candidate_results.iter() {
        if *candidate_id != candidate_data.candidate_id {
            return Err(anyhow!(
                "Invalid {tally_sheet_ref}: inconsistent candidate result, key {candidate_id} != candidate_id {}",
                candidate_data.candidate_id
            )
            .into());
        }
        if !candidates_map.contains_key(&candidate_data.candidate_id) {
            return Err(anyhow!(
                "Invalid {tally_sheet_ref}: can't find candidate {}",
                candidate_data.candidate_id
            )
            .into());
        }
    }
    Ok(())
}
