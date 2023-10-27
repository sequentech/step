// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{
    fs,
    path::{Path, PathBuf},
};

use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};

use super::counting_algorithm::{plurality_at_large::PluralityAtLarge, CountingAlgorithm};

pub enum TallyType {
    PluralityAtLarge,
}

pub struct Tally {
    pub id: TallyType,
    pub contest: Contest,
    pub ballots: Vec<DecodedVoteContest>,
}

impl Tally {
    pub fn new(contest_config: &Path, ballots_file: &Path) -> Result<Self> {
        let contest = Self::get_contest(contest_config)?;
        let ballots = Self::get_ballots(ballots_file)?;
        let id = Self::get_tally_type(&contest)?;

        Ok(Self {
            id,
            contest,
            ballots,
        })
    }

    fn get_tally_type(contest: &Contest) -> Result<TallyType> {
        if let Some(val) = &contest.counting_algorithm {
            if val == "plurality-at-large" {
                return Ok(TallyType::PluralityAtLarge);
            } else {
                return Err(Error::TallyTypeNotImplemented(val.to_owned()));
            }
        }

        Err(Error::TallyTypeNotFound)
    }

    fn get_contest(file: &Path) -> Result<Contest> {
        let file = fs::File::open(&file).map_err(|e| Error::IO(PathBuf::from(file), e))?;
        let res: Contest = serde_json::from_reader(file)?;

        Ok(res)
    }

    fn get_ballots(file: &Path) -> Result<Vec<DecodedVoteContest>> {
        let file = fs::File::open(&file).map_err(|e| Error::IO(PathBuf::from(file), e))?;
        let res: Vec<DecodedVoteContest> = serde_json::from_reader(file)?;

        Ok(res)
    }
}

pub fn create_tally(contest_config: &Path, ballots_file: &Path) -> Result<Box<dyn CountingAlgorithm>> {
    let tally = Tally::new(contest_config, ballots_file)?;

    let ca = match tally.id {
        TallyType::PluralityAtLarge => PluralityAtLarge::new(tally),
    };

    Ok(Box::new(ca))
}

#[derive(Debug)]
pub enum Error {
    TallyTypeNotFound,
    TallyTypeNotImplemented(String),
    IO(PathBuf, std::io::Error),
    Serde(serde_json::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl From<serde_json::Error> for Error {
    fn from(val: serde_json::Error) -> Self {
        Self::Serde(val)
    }
}

impl std::error::Error for Error {}
