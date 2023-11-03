// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::error::{Error, Result};
use crate::cli::{state::Stage, CliRun};
use sequent_core::ballot::{BallotStyle, Contest};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const PREFIX_ELECTION: &str = "election__";
pub const PREFIX_CONTEST: &str = "contest__";
pub const PREFIX_REGION: &str = "region__";

pub const DEFAULT_DIR_CONFIGS: &str = "default/configs";
pub const DEFAULT_DIR_BALLOTS: &str = "default/ballots";

pub const ELECTION_CONFIG_FILE: &str = "election-config.json";
pub const CONTEST_CONFIG_FILE: &str = "contest-config.json";
pub const BALLOTS_FILE: &str = "ballots.csv";

#[derive(Debug)]
pub struct PipeInputs {
    pub cli: CliRun,
    pub root_path_config: PathBuf,
    pub root_path_ballots: PathBuf,
    pub stage: Stage,
    pub election_list: Vec<ElectionConfig>,
}

impl PipeInputs {
    pub fn new(cli: CliRun, stage: Stage) -> Result<Self> {
        let root_path_config = &cli.input_dir.join(DEFAULT_DIR_CONFIGS);
        let root_path_ballots = &cli.input_dir.join(DEFAULT_DIR_BALLOTS);
        let election_list = Self::read_input_dir_config(root_path_config.as_path())?;

        Ok(Self {
            cli,
            root_path_config: root_path_config.to_path_buf(),
            root_path_ballots: root_path_ballots.to_path_buf(),
            stage,
            election_list,
        })
    }

    pub fn build_path(
        root: &Path,
        election_id: &Uuid,
        contest_id: &Uuid,
        region_id: Option<&Uuid>,
    ) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(root);
        path.push(format!("{}{}", PREFIX_ELECTION, election_id));
        path.push(format!("{}{}", PREFIX_CONTEST, contest_id));

        if let Some(region_id) = region_id {
            path.push(format!("{}{}", PREFIX_REGION, region_id));
        }

        path
    }

    fn read_input_dir_config(input_dir: &Path) -> Result<Vec<ElectionConfig>> {
        let entries = fs::read_dir(input_dir)?;

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
        let config_path = path.join(ELECTION_CONFIG_FILE);
        if !config_path.exists() {
            return Err(Error::ElectionConfigNotFound(election_id));
        }
        let config_file =
            fs::File::open(&config_path).map_err(|e| Error::IO(config_path.clone(), e))?;
        let ballot_style: BallotStyle = serde_json::from_reader(config_file)?;

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
            ballot_style,
            contest_list: configs,
            path: path.to_path_buf(),
        })
    }

    fn read_contest_list_config(
        path: &Path,
        election_id: Uuid,
    ) -> Result<ContestForElectionConfig> {
        let contest_id =
            Self::parse_path_components(path, PREFIX_CONTEST).ok_or(Error::IDNotFound)?;
        let config_path = path.join(CONTEST_CONFIG_FILE);
        if !config_path.exists() {
            return Err(Error::ContestConfigNotFound(contest_id));
        }
        let config_file =
            fs::File::open(&config_path).map_err(|e| Error::IO(config_path.clone(), e))?;
        let contest: Contest = serde_json::from_reader(config_file)?;

        let entries = fs::read_dir(path)?;
        let mut configs = vec![];
        for entry in entries {
            let path_region = entry?.path();
            if path_region.is_dir() {
                let region_id = Self::parse_path_components(&path_region, PREFIX_REGION)
                    .ok_or(Error::IDNotFound)?;
                configs.push(Region {
                    id: region_id,
                    contest_id,
                    path: path_region,
                });
            }
        }

        Ok(ContestForElectionConfig {
            id: contest_id,
            election_id,
            contest,
            region_list: configs,
            path: path.to_path_buf(),
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
    pub ballot_style: BallotStyle,
    pub contest_list: Vec<ContestForElectionConfig>,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ContestForElectionConfig {
    pub id: Uuid,
    pub election_id: Uuid,
    pub contest: Contest,
    pub region_list: Vec<Region>,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct Region {
    pub id: Uuid,
    pub contest_id: Uuid,
    pub path: PathBuf,
}
