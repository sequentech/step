// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::counting_algorithm::{plurality_at_large::PluralityAtLarge, CountingAlgorithm};
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
    pub ballots: Vec<(DecodedVoteContest, Option<u64>)>, // (ballot, weight)
    pub census: u64,
    pub auditable_votes: u64,
    pub tally_sheet_results: Vec<ContestResult>,
    pub tally_results: Vec<ContestResult>,
}

impl Tally {
    #[instrument(err, skip(contest, tally_results), name = "Tally::new")]
    pub fn new(
        contest: &Contest,
        ballots_files: Vec<(PathBuf, Option<u64>)>,
        census: u64,
        auditable_votes: u64,
        tally_sheet_results: Vec<ContestResult>,
        tally_results: Vec<ContestResult>,
    ) -> Result<Self> {
        let contest = contest.clone();
        let ballots_with_weights: Vec<(DecodedVoteContest, Option<u64>)> = Self::get_ballots(ballots_files)?;
        let id = Self::get_tally_type(&contest)?;

        Ok(Self {
            id,
            contest,
            ballots: ballots_with_weights,
            census,
            auditable_votes,
            tally_sheet_results,
            tally_results,
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
    fn get_ballots(files: Vec<(PathBuf,Option<u64>)>) -> Result<Vec<(DecodedVoteContest,Option<u64>)>> {
        let mut res = vec![];

        for (f, weight) in files {
            let f = fs::File::open(&f).map_err(|e| PipesError::FileAccess(f, e))?;
            let votes: Vec<DecodedVoteContest> = parse_file(f)?;
            let votes_with_weight: Vec<(DecodedVoteContest, Option<u64>)> = votes.into_iter().map(|v| (v, weight)).collect();
            res.push(votes_with_weight);
        }

        Ok(res
            .into_iter()
            .flatten()
            .collect::<Vec<(DecodedVoteContest, Option<u64>)>>())
    }
}

#[instrument(err, skip_all)]
pub fn process_tally_sheet(
    tally_sheet: &TallySheet,
    contest: &Contest,
    vote_weight: Option<u64>,
) -> Result<ContestResult> {
    let weight = vote_weight.unwrap_or(1);
    let Some(content) = tally_sheet.content.clone() else {
        return Err("missing tally sheet content".into());
    };
    let invalid_votes = content.invalid_votes.unwrap_or(Default::default());

    let count_invalid_votes = InvalidVotes {
        explicit: invalid_votes.explicit_invalid.unwrap_or(0) * weight,
        implicit: invalid_votes.implicit_invalid.unwrap_or(0) * weight,
    };
    let count_invalid: u64 = count_invalid_votes.explicit + count_invalid_votes.implicit;
    let count_blank: u64 = content.total_blank_votes.unwrap_or(0) * weight;

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

    let count_valid: u64 = candidate_results
        .iter()
        .map(|candidate_result| candidate_result.total_count * weight)
        .sum();

    let total_votes = count_valid + count_invalid;

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
    ballots_files: Vec<(PathBuf, Option<u64>)>, // (path, weight)
    census: u64,
    auditable_votes: u64,
    tally_sheet_results: Vec<ContestResult>,
    tally_results: Vec<ContestResult>,
) -> Result<Box<dyn CountingAlgorithm>> {
    let ballots_files: Vec<(PathBuf, Option<u64>)> = ballots_files
        .iter()
        .filter(|(f, _weight)| {
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
        .map(|(p, weight)|  (PathBuf::from(p.as_path()), weight.clone()))
        .collect();

    let tally = Tally::new(
        contest,
        ballots_files,
        census,
        auditable_votes,
        tally_sheet_results,
        tally_results,
    )?;

    let counting_algorithm = match tally.id {
        TallyType::PluralityAtLarge => PluralityAtLarge::new(tally),
    };

    Ok(Box::new(counting_algorithm))
}
