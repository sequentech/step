// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
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
    let Some(content) = tally_sheet.content.clone() else {
        return Err(anyhow!("Invalid tally sheet {:?}, content missing", tally_sheet).into());
    };
    if content.total_votes > content.census {
        return Err(anyhow!(
            "Invalid tally sheet {:?}, total_votes higher than census",
            tally_sheet
        )
        .into());
    }
    // Counters are u64 and release builds wrap on overflow, so every sum is
    // checked: a sum that does not fit is never equal to a counter.
    let invalid_votes = content.invalid_votes.unwrap_or(Default::default());
    let total_invalid_votes_calculated = invalid_votes
        .explicit_invalid
        .unwrap_or(0)
        .checked_add(invalid_votes.implicit_invalid.unwrap_or(0));
    let total_invalid_votes = invalid_votes.total_invalid.unwrap_or(0);
    if total_invalid_votes_calculated != Some(total_invalid_votes) {
        return Err(anyhow!(
            "Invalid tally sheet {:?}, inconsistent total invalid votes",
            tally_sheet
        )
        .into());
    }
    let total_votes = content.total_votes.unwrap_or(0);
    let total_valid_votes = content.total_valid_votes.unwrap_or(0);
    let total_blank_votes = content.total_blank_votes.unwrap_or(0);
    if total_invalid_votes.checked_add(total_valid_votes) != Some(total_votes) {
        return Err(anyhow!(
            "Invalid tally sheet {:?}, inconsistent total votes",
            tally_sheet
        )
        .into());
    }
    // The candidate votes are added up again when the tally aggregates the
    // sheet, so their sum must fit a counter too.
    let Some(total_valid_votes_calc) = content
        .candidate_results
        .values()
        .try_fold(0u64, |sum, candidate_result| {
            sum.checked_add(candidate_result.total_votes.unwrap_or(0))
        })
    else {
        return Err(anyhow!(
            "Invalid tally sheet {:?}, candidate votes add up to more than {}",
            tally_sheet,
            u64::MAX
        )
        .into());
    };

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
    for (candidate_id, candidate_data) in content.candidate_results.iter() {
        if *candidate_id != candidate_data.candidate_id {
            return Err(anyhow!(
                "Invalid tally sheet {:?}, inconsistent candidate result {:?}, {}",
                tally_sheet,
                candidate_data,
                candidate_id
            )
            .into());
        }
        if !candidates_map.contains_key(&candidate_data.candidate_id) {
            return Err(anyhow!(
                "Invalid tally sheet {:?}, can't find candidate {:?}",
                tally_sheet,
                candidate_data
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::tally_sheets::{AreaContestResults, CandidateResults, InvalidVotes};

    fn contest() -> Contest {
        Contest {
            id: "contest".to_string(),
            candidates: ["candidate-a", "candidate-b"]
                .into_iter()
                .map(|id| Candidate {
                    id: id.to_string(),
                    ..Candidate::default()
                })
                .collect(),
            ..Contest::default()
        }
    }

    /// A consistent sheet: 10 votes, 1 implicitly invalid, 9 valid of which
    /// 2 are blank, and 4 + 3 candidate votes.
    fn content() -> AreaContestResults {
        AreaContestResults {
            area_id: "area".to_string(),
            contest_id: "contest".to_string(),
            total_votes: Some(10),
            total_valid_votes: Some(9),
            invalid_votes: Some(InvalidVotes {
                total_invalid: Some(1),
                implicit_invalid: Some(1),
                explicit_invalid: Some(0),
            }),
            total_blank_votes: Some(2),
            census: Some(10),
            candidate_results: [("candidate-a", 4), ("candidate-b", 3)]
                .into_iter()
                .map(|(id, votes)| {
                    (
                        id.to_string(),
                        CandidateResults {
                            candidate_id: id.to_string(),
                            total_votes: Some(votes),
                        },
                    )
                })
                .collect(),
        }
    }

    fn tally_sheet(content: AreaContestResults) -> TallySheet {
        TallySheet {
            id: "tally-sheet".to_string(),
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            election_id: "election".to_string(),
            contest_id: "contest".to_string(),
            area_id: "area".to_string(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            published_at: None,
            published_by_user_id: None,
            content: Some(content),
            channel: None,
            deleted_at: None,
            created_by_user_id: "user".to_string(),
        }
    }

    fn validation_error(content: AreaContestResults) -> String {
        validate_tally_sheet(&tally_sheet(content), &contest())
            .map(|_| "the sheet should be refused".to_string())
            .unwrap_or_else(|error| format!("{error:?}"))
    }

    #[test]
    fn accepts_a_consistent_sheet() {
        assert!(validate_tally_sheet(&tally_sheet(content()), &contest()).is_ok());
    }

    #[test]
    fn refuses_invalid_votes_whose_sum_wraps() {
        // u64::MAX + 1 wraps to 0 = total_invalid.
        let mut content = content();
        content.total_votes = Some(9);
        content.invalid_votes = Some(InvalidVotes {
            total_invalid: Some(0),
            implicit_invalid: Some(u64::MAX),
            explicit_invalid: Some(1),
        });
        assert!(validation_error(content).contains("inconsistent total invalid votes"));
    }

    #[test]
    fn refuses_vote_totals_whose_sum_wraps() {
        // u64::MAX + 2 wraps to 1 = total_votes.
        let mut content = content();
        content.total_votes = Some(1);
        content.total_valid_votes = Some(u64::MAX);
        content.invalid_votes = Some(InvalidVotes {
            total_invalid: Some(2),
            implicit_invalid: Some(2),
            explicit_invalid: Some(0),
        });
        assert!(validation_error(content).contains("inconsistent total votes"));
    }

    #[test]
    fn refuses_candidate_votes_whose_sum_wraps() {
        // [2^64 - 100, 200] wraps to 100.
        let mut content = content();
        for (id, votes) in [("candidate-a", u64::MAX - 99), ("candidate-b", 200)] {
            content
                .candidate_results
                .get_mut(id)
                .expect("candidate")
                .total_votes = Some(votes);
        }
        assert!(validation_error(content).contains("candidate votes add up to more than"));
    }
}
