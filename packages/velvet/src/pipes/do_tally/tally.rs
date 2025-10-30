// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::counting_algorithm::Error as CntAlgError;
use super::counting_algorithm::{
    instant_runoff::InstantRunoff, plurality_at_large::PluralityAtLarge, CountingAlgorithm,
};
use super::error::{Error, Result};
use super::{CandidateResult, ContestResult, InvalidVotes};
use crate::pipes::error::Error as PipesError;
use crate::pipes::pipe_name::PipeName;
use crate::utils::parse_file;
use sequent_core::ballot::{ContestPresentation, Weight};
use sequent_core::types::{
    ceremonies::{CountingAlgType, ScopeOperation, TallyOperation},
    hasura::core::TallySheet,
};
use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};
use std::cmp;
use std::{fs, path::PathBuf};
use strum_macros::{Display, EnumString};
use tracing::instrument;

pub struct Tally {
    pub id: CountingAlgType,
    pub scope_operation: ScopeOperation,
    pub contest: Contest,
    pub ballots: Vec<(DecodedVoteContest, Weight)>,
    pub census: u64,
    pub auditable_votes: u64,
    pub tally_sheet_results: Vec<ContestResult>,
    pub tally_results: Vec<ContestResult>,
}

impl Tally {
    #[instrument(err, skip(contest, tally_results), name = "Tally::new")]
    pub fn new(
        contest: &Contest,
        scope_operation: ScopeOperation,
        ballots_files: Vec<(PathBuf, Weight)>,
        census: u64,
        auditable_votes: u64,
        tally_sheet_results: Vec<ContestResult>,
        tally_results: Vec<ContestResult>,
    ) -> Result<Self> {
        let contest = contest.clone();
        let ballots_with_weights: Vec<(DecodedVoteContest, Weight)> =
            Self::get_ballots(ballots_files)?;
        let id = Self::get_tally_type(&contest)?;

        Ok(Self {
            id,
            scope_operation,
            contest,
            ballots: ballots_with_weights,
            census,
            auditable_votes,
            tally_sheet_results,
            tally_results,
        })
    }

    #[instrument(err, skip_all)]
    fn get_tally_type(contest: &Contest) -> Result<CountingAlgType> {
        contest
            .counting_algorithm
            .ok_or_else(|| Box::new(Error::TallyTypeNotFound) as Box<dyn std::error::Error>)
    }

    #[instrument(err, skip_all)]
    fn get_ballots(files: Vec<(PathBuf, Weight)>) -> Result<Vec<(DecodedVoteContest, Weight)>> {
        let mut res = vec![];

        for (f, weight) in files {
            let f = fs::File::open(&f).map_err(|e| PipesError::FileAccess(f, e))?;
            let votes: Vec<DecodedVoteContest> = parse_file(f)?;
            let votes_with_weight: Vec<(DecodedVoteContest, Weight)> =
                votes.into_iter().map(|v| (v, weight)).collect();
            res.push(votes_with_weight);
        }

        Ok(res
            .into_iter()
            .flatten()
            .collect::<Vec<(DecodedVoteContest, Weight)>>())
    }

    #[instrument(skip_all)]
    pub fn aggregate_results(&self) -> Result<ContestResult, CntAlgError> {
        if self.tally_results.is_empty() {
            return Err(CntAlgError::EmptyTallyResults);
        }
        let mut contest_result = ContestResult::default();
        contest_result.contest = self.contest.clone();
        let aggregated = self
            .tally_results
            .iter()
            .fold(contest_result, |acc, x| acc.aggregate(x, true));
        Ok(aggregated)
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
    let count_invalid: u64 = count_invalid_votes.explicit + count_invalid_votes.implicit;
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

    let count_valid: u64 = candidate_results
        .iter()
        .map(|candidate_result| candidate_result.total_count)
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
        process_results: None,
    };
    Ok(contest_result.calculate_percentages())
}

#[instrument(err, skip_all)]
pub fn create_tally(
    contest: &Contest,
    scope_operation: ScopeOperation,
    ballots_files: Vec<(PathBuf, Weight)>, // (path, weight)
    census: u64,
    auditable_votes: u64,
    tally_sheet_results: Vec<ContestResult>,
    tally_results: Vec<ContestResult>,
) -> Result<Box<dyn CountingAlgorithm>> {
    let ballots_files: Vec<(PathBuf, Weight)> = ballots_files
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
        .map(|(p, weight)| (PathBuf::from(p.as_path()), weight.clone()))
        .collect();

    let tally = Tally::new(
        contest,
        scope_operation,
        ballots_files,
        census,
        auditable_votes,
        tally_sheet_results,
        tally_results,
    )?;

    match tally.id {
        CountingAlgType::PluralityAtLarge => Ok(Box::new(PluralityAtLarge::new(tally))),
        CountingAlgType::InstantRunoff => Ok(Box::new(InstantRunoff::new(tally))),
        _ => Err(Box::new(Error::TallyTypeNotImplemented(
            tally.id.to_string(),
        ))),
    }
}
