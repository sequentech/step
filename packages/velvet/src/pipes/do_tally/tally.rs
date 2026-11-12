// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::counting_algorithm::{plurality_at_large::PluralityAtLarge, CountingAlgorithm};
use super::do_tally::checked_count_sum;
use super::error::{Error, Result};
use super::{CandidateResult, ContestResult, InvalidVotes};
use crate::pipes::error::Error as PipesError;
use crate::pipes::pipe_name::PipeName;
use crate::utils::parse_file;
use sequent_core::ballot::ContestPresentation;
use sequent_core::types::hasura::core::TallySheet;
use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};
use std::cmp;
use std::{fs, path::PathBuf};
use tracing::instrument;

pub enum TallyType {
    PluralityAtLarge,
}

pub struct Tally {
    pub id: TallyType,
    pub contest: Contest,
    pub ballots: Vec<DecodedVoteContest>,
    pub census: u64,
    pub auditable_votes: u64,
    pub tally_sheet_results: Vec<ContestResult>,
}

impl Tally {
    #[instrument(err, skip(contest), name = "Tally::new")]
    pub fn new(
        contest: &Contest,
        ballots_files: Vec<PathBuf>,
        census: u64,
        auditable_votes: u64,
        tally_sheet_results: Vec<ContestResult>,
    ) -> Result<Self> {
        let contest = contest.clone();
        let ballots = Self::get_ballots(ballots_files)?;
        let id = Self::get_tally_type(&contest)?;

        Ok(Self {
            id,
            contest,
            ballots,
            census,
            auditable_votes,
            tally_sheet_results,
        })
    }

    #[instrument(err, skip_all)]
    fn get_tally_type(contest: &Contest) -> Result<TallyType> {
        if let Some(val) = &contest.counting_algorithm {
            if val == "plurality-at-large" {
                return Ok(TallyType::PluralityAtLarge);
            } else {
                return Err(Box::new(Error::TallyTypeNotImplemented(val.to_owned())));
            }
        }

        Err(Box::new(Error::TallyTypeNotFound))
    }

    #[instrument(err, skip_all)]
    fn get_ballots(files: Vec<PathBuf>) -> Result<Vec<DecodedVoteContest>> {
        let mut res = vec![];

        for f in files {
            let f = fs::File::open(&f).map_err(|e| PipesError::FileAccess(f, e))?;
            let votes: Vec<DecodedVoteContest> = parse_file(f)?;
            res.push(votes);
        }

        Ok(res
            .into_iter()
            .flatten()
            .collect::<Vec<DecodedVoteContest>>())
    }
}

#[instrument(err, skip_all)]
pub fn process_tally_sheet(tally_sheet: &TallySheet, contest: &Contest) -> Result<ContestResult> {
    let Some(content) = tally_sheet.content.clone() else {
        return Err("missing tally sheet content".into());
    };
    let invalid_votes = content.invalid_votes.unwrap_or(Default::default());

    let count_invalid_votes = InvalidVotes {
        explicit: invalid_votes.explicit_invalid.unwrap_or(0),
        implicit: invalid_votes.implicit_invalid.unwrap_or(0),
    };
    let count_invalid = checked_count_sum(
        [count_invalid_votes.explicit, count_invalid_votes.implicit],
        "total_invalid",
    )?;
    let count_blank: u64 = content.total_blank_votes.unwrap_or(0);

    let candidate_results = content
        .candidate_results
        .values()
        .map(|candidate| -> Result<CandidateResult> {
            let Some(found_candidate) = contest
                .candidates
                .iter()
                .find(|c| candidate.candidate_id == c.id)
            else {
                return Err("can't find Candidate".into());
            };

            Ok(CandidateResult {
                candidate: found_candidate.clone(),
                percentage_votes: 0.0,
                total_count: candidate.total_votes.unwrap_or(0),
            })
        })
        .collect::<Result<Vec<CandidateResult>>>()?;

    let count_valid = checked_count_sum(
        candidate_results
            .iter()
            .map(|candidate_result| candidate_result.total_count),
        "candidate votes",
    )?;

    let total_votes = checked_count_sum([count_valid, count_invalid], "total_votes")?;

    let contest_result = ContestResult {
        contest: contest.clone(),
        census: content.census.unwrap_or(0),
        percentage_census: 100.0,
        auditable_votes: 0,
        percentage_auditable_votes: 0.0,
        total_votes: total_votes,
        percentage_total_votes: 0.0,
        total_valid_votes: count_valid,
        percentage_total_valid_votes: 0.0,
        total_invalid_votes: count_invalid,
        percentage_total_invalid_votes: 0.0,
        total_blank_votes: count_blank,
        percentage_total_blank_votes: 0.0,
        percentage_invalid_votes_explicit: 0.0,
        percentage_invalid_votes_implicit: 0.0,
        invalid_votes: count_invalid_votes,
        candidate_result: candidate_results,
        extended_metrics: None,
    };
    Ok(contest_result.calculate_percentages())
}

#[instrument(err, skip_all)]
pub fn create_tally(
    contest: &Contest,
    ballots_files: Vec<PathBuf>,
    census: u64,
    auditable_votes: u64,
    tally_sheet_results: Vec<ContestResult>,
) -> Result<Box<dyn CountingAlgorithm>> {
    let ballots_files = ballots_files
        .iter()
        .filter(|f| {
            let exist = f.exists();
            if !exist {
                println!(
                    "[{}] File not found: {} -- Not processed",
                    PipeName::DoTally.as_ref(),
                    f.display()
                )
            }
            exist
        })
        .map(|p| PathBuf::from(p.as_path()))
        .collect();

    let tally = Tally::new(
        contest,
        ballots_files,
        census,
        auditable_votes,
        tally_sheet_results,
    )?;

    let counting_algorithm = match tally.id {
        TallyType::PluralityAtLarge => PluralityAtLarge::new(tally),
    };

    Ok(Box::new(counting_algorithm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::Candidate;
    use sequent_core::types::tally_sheets::{
        AreaContestResults, CandidateResults, InvalidVotes as TallySheetInvalidVotes,
    };

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

    fn tally_sheet(candidate_votes: [u64; 2], explicit: u64, implicit: u64) -> TallySheet {
        let candidate_results = ["candidate-a", "candidate-b"]
            .into_iter()
            .zip(candidate_votes)
            .map(|(id, votes)| {
                (
                    id.to_string(),
                    CandidateResults {
                        candidate_id: id.to_string(),
                        total_votes: Some(votes),
                    },
                )
            })
            .collect();
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
            content: Some(AreaContestResults {
                area_id: "area".to_string(),
                contest_id: "contest".to_string(),
                total_votes: None,
                total_valid_votes: None,
                invalid_votes: Some(TallySheetInvalidVotes {
                    total_invalid: None,
                    implicit_invalid: Some(implicit),
                    explicit_invalid: Some(explicit),
                }),
                total_blank_votes: Some(1),
                census: Some(20),
                candidate_results,
            }),
            channel: None,
            deleted_at: None,
            created_by_user_id: "user".to_string(),
        }
    }

    #[test]
    fn process_tally_sheet_counts_a_consistent_sheet() {
        let result = process_tally_sheet(&tally_sheet([4, 3], 1, 2), &contest())
            .expect("the sheet should be processed");

        assert_eq!(result.total_valid_votes, 7);
        assert_eq!(result.total_invalid_votes, 3);
        assert_eq!(result.total_votes, 10);
        assert_eq!(result.total_blank_votes, 1);
    }

    #[test]
    fn process_tally_sheet_rejects_candidate_votes_beyond_the_counter_range() {
        // [2^64 - 100, 200] would wrap to 100.
        let sheet = tally_sheet([u64::MAX - 99, 200], 0, 0);

        assert!(process_tally_sheet(&sheet, &contest()).is_err());
    }

    #[test]
    fn process_tally_sheet_rejects_invalid_votes_beyond_the_counter_range() {
        let sheet = tally_sheet([4, 3], u64::MAX, 1);

        assert!(process_tally_sheet(&sheet, &contest()).is_err());
    }

    #[test]
    fn process_tally_sheet_rejects_vote_totals_beyond_the_counter_range() {
        let sheet = tally_sheet([u64::MAX, 0], 0, 1);

        assert!(process_tally_sheet(&sheet, &contest()).is_err());
    }
}
