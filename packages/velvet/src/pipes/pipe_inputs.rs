use super::error::{Error, Result};
use crate::cli::{state::Stage, CliRun};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const PREFIX_ELECTION: &str = "election__";
pub const PREFIX_CONTEST: &str = "contest__";

pub const DEFAULT_DIR_CONFIGS: &str = "default/configs";
pub const DEFAULT_DIR_BALLOTS: &str = "default/ballots";

pub const ELECTION_CONFIG_FILE: &str = "election-config.json";
pub const CONTEST_CONFIG_FILE: &str = "contest-config.json";
pub const BALLOTS_FILE: &str = "ballots.csv";

pub trait PipeInputsRead {
    // read input_dir into PipeInput
    fn read_input_dir_config(&self) -> Result<()>;
}

#[derive(Debug)]
pub struct PipeInputs {
    pub cli: CliRun,
    pub stage: Stage,
    // TODO: Election Event Config
    pub election_list: Vec<ElectionConfig>,
}

impl PipeInputs {
    pub fn new(cli: CliRun, stage: Stage) -> Result<Self> {
        let election_list = Self::read_input_dir_config(&cli.input_dir)?;

        Ok(Self {
            cli,
            stage,
            election_list,
        })
    }

    pub fn get_path_for_contest(
        &self,
        root: &Path,
        election_id: &Uuid,
        contest_id: &Uuid,
    ) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(root);
        path.push(DEFAULT_DIR_BALLOTS);
        path.push(format!("{}{}", PREFIX_ELECTION, election_id));
        path.push(format!("{}{}", PREFIX_CONTEST, contest_id));

        path
    }

    fn read_input_dir_config(input_dir: &Path) -> Result<Vec<ElectionConfig>> {
        let entries = fs::read_dir(input_dir.join(DEFAULT_DIR_CONFIGS))?;

        let mut configs = vec![];
        for entry in entries {
            let config = Self::read_election_list_config(&entry?.path())?;
            configs.push(config);
        }

        Ok(configs)
    }

    fn read_election_list_config(path: &Path) -> Result<ElectionConfig> {
        let entries = fs::read_dir(path)?;

        let election_id =
            Self::parse_path_components(path, PREFIX_ELECTION).ok_or(Error::IDNotFound)?;
        let config = path.join(ELECTION_CONFIG_FILE);
        if !config.exists() {
            return Err(Error::ElectionConfigNotFound(election_id));
        }

        let mut configs = vec![];
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                let config = Self::read_contest_list_config(&path, election_id)?;
                configs.push(config);
            }
        }

        Ok(ElectionConfig {
            id: election_id,
            config: config.to_owned(),
            contest_list: configs,
        })
    }

    fn read_contest_list_config(
        path: &Path,
        election_id: Uuid,
    ) -> Result<ContestForElectionConfig> {
        let contest_id =
            Self::parse_path_components(path, PREFIX_CONTEST).ok_or(Error::IDNotFound)?;
        let config = path.join(CONTEST_CONFIG_FILE);
        if !config.exists() {
            return Err(Error::ContestConfigNotFound(contest_id));
        }

        Ok(ContestForElectionConfig {
            id: contest_id,
            election_id,
            config: config.to_owned(),
        })
    }

    fn parse_path_components(path: &Path, prefix: &str) -> Option<Uuid> {
        for component in path.components() {
            let part = component.as_os_str().to_string_lossy();

            if let Some(res) = part.strip_prefix(prefix) {
                return Uuid::parse_str(res).ok();
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct ElectionConfig {
    pub id: Uuid,
    pub config: PathBuf,
    pub contest_list: Vec<ContestForElectionConfig>,
}

#[derive(Debug)]
pub struct ContestForElectionConfig {
    pub id: Uuid,
    pub election_id: Uuid,
    pub config: PathBuf,
    // TODO: areas
}
