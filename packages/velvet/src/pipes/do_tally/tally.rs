// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::counting_algorithm::{plurality_at_large::PluralityAtLarge, CountingAlgorithm};
use super::error::{Error, Result};
use super::{CandidateResult, ContestResult, InvalidVotes};
use crate::pipes::error::Error as PipesError;
use crate::pipes::pipe_name::PipeName;
use crate::utils::parse_file;
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
}

impl Tally {
    #[instrument(skip(contest))]
    pub fn new(contest: &Contest, ballots_files: Vec<PathBuf>, census: u64) -> Result<Self> {
        let contest = contest.clone();
        let ballots = Self::get_ballots(ballots_files)?;
        let id = Self::get_tally_type(&contest)?;

        Ok(Self {
            id,
            contest,
            ballots,
            census,
        })
    }

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
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
    let total_votes_base = cmp::max(1, total_votes) as f64;

    let census_base = cmp::max(1, content.census.unwrap_or(0)) as f64;
    let percentage_total_votes = (total_votes as f64) * 100.0 / census_base;
    let percentage_total_valid_votes = (count_valid as f64 * 100.0) / total_votes_base;
    let percentage_total_invalid_votes = (count_invalid as f64 * 100.0) / total_votes_base;
    let percentage_total_blank_votes = (count_blank as f64 * 100.0) / total_votes_base;
    let percentage_invalid_votes_explicit =
        (count_invalid_votes.explicit as f64 * 100.0) / total_votes_base;
    let percentage_invalid_votes_implicit =
        (count_invalid_votes.implicit as f64 * 100.0) / total_votes_base;

    let contest_result = ContestResult {
        contest: contest.clone(),
        census: content.census.unwrap_or(0),
        percentage_census: 100.0,
        total_votes: total_votes,
        percentage_total_votes: percentage_total_votes.clamp(0.0, 100.0),
        total_valid_votes: count_valid,
        percentage_total_valid_votes: percentage_total_valid_votes.clamp(0.0, 100.0),
        total_invalid_votes: count_invalid,
        percentage_total_invalid_votes: percentage_total_invalid_votes.clamp(0.0, 100.0),
        total_blank_votes: count_blank,
        percentage_total_blank_votes: percentage_total_blank_votes.clamp(0.0, 100.0),
        percentage_invalid_votes_explicit: percentage_invalid_votes_explicit.clamp(0.0, 100.0),
        percentage_invalid_votes_implicit: percentage_invalid_votes_implicit.clamp(0.0, 100.0),
        invalid_votes: count_invalid_votes,
        candidate_result: candidate_results,
    };
    Ok(contest_result)
}

#[instrument(skip_all)]
pub fn create_tally(
    contest: &Contest,
    ballots_files: Vec<PathBuf>,
    census: u64,
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

    let tally = Tally::new(contest, ballots_files, census)?;

    let counting_algorithm = match tally.id {
        TallyType::PluralityAtLarge => PluralityAtLarge::new(tally),
    };

    Ok(Box::new(counting_algorithm))
}
