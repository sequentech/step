use std::{
    fs,
    path::{Path, PathBuf},
};

use super::tally::{plurality_at_large::PluralityAtLargeTally, Tally};
use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};

pub enum VotingSystemId {
    PluralityAtLarge,
}

pub struct VotingSystem {
    pub id: VotingSystemId,
    pub contest: Contest,
    pub ballots: Vec<DecodedVoteContest>,
}

impl VotingSystem {
    pub fn new(contest_config: &Path, ballots_file: &Path) -> Result<Self> {
        let contest = Self::get_contest(contest_config)?;
        let ballots = Self::get_ballots(ballots_file)?;
        let id = Self::get_voting_system_id(&contest)?;

        Ok(Self {
            id,
            contest,
            ballots,
        })
    }

    fn get_voting_system_id(contest: &Contest) -> Result<VotingSystemId> {
        if let Some(val) = &contest.counting_algorithm {
            if val == "plurality-at-large" {
                return Ok(VotingSystemId::PluralityAtLarge);
            } else {
                return Err(Error::VotingSystemNotImplemented(val.to_owned()));
            }
        }

        Err(Error::VotingSystemNotFound)
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

pub fn create_tally(contest_config: &Path, ballots_file: &Path) -> Result<Box<dyn Tally>> {
    let vs = VotingSystem::new(contest_config, ballots_file)?;

    let tally = match vs.id {
        VotingSystemId::PluralityAtLarge => PluralityAtLargeTally::new(vs),
    };

    Ok(Box::new(tally))
}

#[derive(Debug)]
pub enum Error {
    VotingSystemNotFound,
    VotingSystemNotImplemented(String),
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
