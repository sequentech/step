// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use anyhow::anyhow;
use sequent_core::ballot::{Candidate, Contest};
use sequent_core::services::tally_sheet_validation::validate_area_contest_results;
use sequent_core::types::hasura::core::TallySheet;
use std::collections::HashMap;
use tracing::instrument;

#[instrument(skip_all, err)]
pub fn validate_tally_sheet(tally_sheet: &TallySheet, contest: &Contest) -> Result<()> {
    let tally_sheet_ref = format!(
        "tally sheet {} (area {}, contest {})",
        tally_sheet.id, tally_sheet.area_id, tally_sheet.contest_id
    );
    if contest.is_acclaimed() {
        return Err(anyhow!(
            "Invalid {tally_sheet_ref}: acclaimed contests cannot have tally sheets"
        )
        .into());
    }
    let Some(content) = tally_sheet.content.clone() else {
        return Err(anyhow!("Invalid {tally_sheet_ref}: content missing").into());
    };

    // The numeric invariants are shared with the import pipeline, which
    // validates the very sheets this later tallies, so both go through one
    // implementation: a sheet accepted at import must not be rejected here.
    // In particular the candidate-vote total is bounded by a range rather
    // than an equality, since a "vote for N" contest legitimately carries
    // more marks than ballots.
    let errors = validate_area_contest_results(&content, Some(contest.max_marks_per_ballot()));
    if let Some(error) = errors.first() {
        return Err(anyhow!("Invalid {tally_sheet_ref}: {}", error.message).into());
    }

    // Structural checks below are specific to this path: unlike the import
    // pipeline, which resolves candidates by external id while parsing,
    // here the sheet is already stored and its keys must still line up with
    // the contest it is about to be tallied against.
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

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::Contest as BallotContest;
    use sequent_core::types::ceremonies::CountingAlgType;
    use sequent_core::types::tally_sheets::{
        AreaContestResults, CandidateResults, InvalidVotes, TallySheetStatus,
    };

    /// A contest with `max_votes` marks allowed per ballot and `candidates`
    /// numbered `cand-1..=candidates`.
    fn contest(max_votes: i64, candidates: usize) -> BallotContest {
        BallotContest {
            max_votes,
            counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
            candidates: (1..=candidates)
                .map(|index| Candidate {
                    id: format!("cand-{index}"),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    fn tally_sheet(
        total_votes: u64,
        total_valid_votes: u64,
        total_blank_votes: u64,
        candidate_votes: &[u64],
    ) -> TallySheet {
        let candidate_results = candidate_votes
            .iter()
            .enumerate()
            .map(|(index, votes)| {
                let candidate_id = format!("cand-{}", index + 1);
                (
                    candidate_id.clone(),
                    CandidateResults {
                        candidate_id,
                        total_votes: Some(*votes),
                    },
                )
            })
            .collect();
        TallySheet {
            id: "sheet-1".to_string(),
            tenant_id: String::new(),
            election_event_id: String::new(),
            election_id: String::new(),
            contest_id: String::new(),
            area_id: String::new(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            reviewed_at: None,
            reviewed_by_user_id: None,
            content: Some(AreaContestResults {
                area_id: String::new(),
                contest_id: String::new(),
                total_votes: Some(total_votes),
                total_valid_votes: Some(total_valid_votes),
                total_blank_votes: Some(total_blank_votes),
                blank_ballots: None,
                census: Some(total_votes),
                invalid_votes: Some(InvalidVotes {
                    total_invalid: Some(0),
                    implicit_invalid: Some(0),
                    explicit_invalid: Some(0),
                }),
                candidate_results,
                annotations: None,
            }),
            channel: None,
            deleted_at: None,
            created_by_user_id: String::new(),
            status: TallySheetStatus::APPROVED,
            version: 1,
            import_id: None,
        }
    }

    #[test]
    fn accepts_a_vote_for_n_sheet_whose_marks_exceed_the_ballot_count() {
        // A vote-for-4 contest: 10 valid ballots of which 2 are blank, so 8
        // non-blank ballots carrying 32 marks — the upper bound of 8 x 4.
        // An equality rule would read this as 10 != 32 + 2 and reject it.
        let result = validate_tally_sheet(&tally_sheet(10, 10, 2, &[8, 8, 8, 8]), &contest(4, 4));

        assert!(result.is_ok(), "vote-for-4 sheet rejected: {result:?}");
    }

    #[test]
    fn rejects_marks_above_what_the_ballots_could_carry() {
        // One mark beyond the 8 x 4 ceiling is still a real error.
        let result = validate_tally_sheet(&tally_sheet(10, 10, 2, &[9, 8, 8, 8]), &contest(4, 4));

        assert!(result.is_err());
    }

    #[test]
    fn still_requires_equality_for_a_single_choice_contest() {
        // With max_votes 1 the range collapses to a single value, so a
        // single-choice contest is still validated strictly.
        let ok = validate_tally_sheet(&tally_sheet(10, 10, 2, &[8]), &contest(1, 1));
        assert!(ok.is_ok(), "single-choice sheet rejected: {ok:?}");

        let bad = validate_tally_sheet(&tally_sheet(10, 10, 2, &[9]), &contest(1, 1));
        assert!(bad.is_err());
    }

    #[test]
    fn rejects_a_stored_tally_sheet_for_an_acclaimed_contest() {
        let mut acclaimed = contest(1, 1);
        acclaimed.is_acclaimed = Some(true);

        let result = validate_tally_sheet(&tally_sheet(10, 10, 0, &[10]), &acclaimed);

        let error = result.expect_err("acclaimed tally sheet must be rejected");
        assert!(error
            .to_string()
            .contains("acclaimed contests cannot have tally sheets"));
    }
}
