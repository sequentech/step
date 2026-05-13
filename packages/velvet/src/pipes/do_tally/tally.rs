// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Pipeline-side tally orchestration.
//!
//! The `Tally` data type, its pure methods, and `process_tally_sheet`
//! now live in `velvet-core`. This module re-exports them and adds the
//! file-loading wrappers (`tally_from_files`, `create_tally`) that read
//! decoded-ballot files from disk before handing off to the pure
//! `Tally::from_ballots` constructor.

pub use velvet_core::counting::{process_tally_sheet, Tally};

use super::counting_algorithm::{
    instant_runoff::InstantRunoff, plurality_at_large::PluralityAtLarge, CountingAlgorithm,
};
use super::error::{Error, Result};
use super::ContestResult;
use crate::pipes::error::Error as PipesError;
use crate::pipes::pipe_name::PipeName;
use crate::utils::parse_file;
use sequent_core::ballot::{Contest, Weight};
use sequent_core::plaintext::DecodedVoteContest;
use sequent_core::types::ceremonies::{CountingAlgType, ScopeOperation};
use std::{fs, path::PathBuf};
use tracing::instrument;

#[instrument(err, skip_all)]
fn get_ballots(files: Vec<(PathBuf, Weight)>) -> Result<Vec<(DecodedVoteContest, Weight)>> {
    let mut res = vec![];

    for (f, weight) in files {
        let f_open = fs::File::open(&f).map_err(|e| PipesError::FileAccess(f, e))?;
        let votes: Vec<DecodedVoteContest> = parse_file(f_open)?;
        let votes_with_weight: Vec<(DecodedVoteContest, Weight)> =
            votes.into_iter().map(|v| (v, weight)).collect();
        res.push(votes_with_weight);
    }

    Ok(res
        .into_iter()
        .flatten()
        .collect::<Vec<(DecodedVoteContest, Weight)>>())
}

/// File-loading constructor for `Tally`. Reads decoded-ballot files from
/// disk and hands the in-memory ballots to `Tally::from_ballots`. Used
/// by the do-tally pipeline and by ballot-images rendering.
#[instrument(err, skip(contest, tally_sheet_results, tally_results), name = "tally_from_files")]
pub fn tally_from_files(
    contest: &Contest,
    scope_operation: ScopeOperation,
    ballots_files: Vec<(PathBuf, Weight)>,
    census: u64,
    auditable_votes: u64,
    tally_sheet_results: Vec<ContestResult>,
    tally_results: Vec<ContestResult>,
) -> Result<Tally> {
    let ballots = get_ballots(ballots_files)?;
    Tally::from_ballots(
        contest,
        scope_operation,
        ballots,
        census,
        auditable_votes,
        tally_sheet_results,
        tally_results,
    )
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
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

    let tally = tally_from_files(
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
