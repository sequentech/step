// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::counting_algorithm::{plurality_at_large::PluralityAtLarge, CountingAlgorithm};
use super::error::{Error, Result};
use crate::pipes::error::Error as PipesError;
use crate::pipes::pipe_name::PipeName;
use crate::utils::parse_file;
use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};
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
