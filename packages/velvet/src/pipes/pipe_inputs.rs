// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::error::{Error, Result};
use crate::{
    cli::{state::Stage, CliRun},
    utils::parse_file,
};
use sequent_core::{
    ballot::{BallotStyle, Contest},
    services::area_tree::TreeNodeArea,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const PREFIX_ELECTION: &str = "election__";
pub const PREFIX_CONTEST: &str = "contest__";
pub const PREFIX_AREA: &str = "area__";
pub const PREFIX_TALLY_SHEET: &str = "tally_sheet__";

pub const DEFAULT_DIR_CONFIGS: &str = "default/configs";
pub const DEFAULT_DIR_BALLOTS: &str = "default/ballots";
pub const DEFAULT_DIR_TALLY_SHEETS: &str = "default/tally_sheets";

pub const ELECTION_CONFIG_FILE: &str = "election-config.json";
pub const CONTEST_CONFIG_FILE: &str = "contest-config.json";
pub const AREA_CONFIG_FILE: &str = "area-config.json";
pub const BALLOTS_FILE: &str = "ballots.csv";

#[derive(Debug)]
pub struct PipeInputs {
    pub cli: CliRun,
    pub root_path_config: PathBuf,
    pub root_path_ballots: PathBuf,
    pub root_path_tally_sheets: PathBuf,
    pub stage: Stage,
    pub election_list: Vec<InputElectionConfig>,
}

impl PipeInputs {
    pub fn new(cli: CliRun, stage: Stage) -> Result<Self> {
        let root_path_config = &cli.input_dir.join(DEFAULT_DIR_CONFIGS);
        let root_path_ballots = &cli.input_dir.join(DEFAULT_DIR_BALLOTS);
        let root_path_tally_sheets = &cli.input_dir.join(DEFAULT_DIR_TALLY_SHEETS);
        let election_list = Self::read_input_dir_config(root_path_config.as_path())?;

        Ok(Self {
            cli,
            root_path_config: root_path_config.to_path_buf(),
            root_path_ballots: root_path_ballots.to_path_buf(),
            root_path_tally_sheets: root_path_tally_sheets.to_path_buf(),
            stage,
            election_list,
        })
    }

    pub fn build_path(
        root: &Path,
        election_id: &Uuid,
        contest_id: Option<&Uuid>,
        area_id: Option<&Uuid>,
    ) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(root);
        path.push(format!("{}{}", PREFIX_ELECTION, election_id));

        if let Some(contest_id) = contest_id {
            path.push(format!("{}{}", PREFIX_CONTEST, contest_id));

            if let Some(area_id) = area_id {
                path.push(format!("{}{}", PREFIX_AREA, area_id));
            }
        }

        path
    }

    pub fn add_tally_sheet_path(root: &Path, tally_sheet_id: &Uuid) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(root);
        path.push(format!("{}{}", PREFIX_TALLY_SHEET, tally_sheet_id));
        path
    }

    fn read_input_dir_config(input_dir: &Path) -> Result<Vec<InputElectionConfig>> {
        let entries = fs::read_dir(input_dir)?;

        let mut configs = vec![];
        for entry in entries {
            let config = Self::read_election_list_config(&entry?.path())?;
            configs.push(config);
        }

        Ok(configs)
    }

    fn read_election_list_config(path: &Path) -> Result<InputElectionConfig> {
        let entries = fs::read_dir(path)?;

        let election_id =
            Self::parse_path_components(path, PREFIX_ELECTION).ok_or(Error::IDNotFound)?;
        let config_path = path.join(ELECTION_CONFIG_FILE);
        if !config_path.exists() {
            return Err(Error::ElectionConfigNotFound(election_id));
        }
        let config_file =
            fs::File::open(&config_path).map_err(|e| Error::FileAccess(config_path.clone(), e))?;

        let election: ElectionConfig = parse_file(config_file)?;

        let mut configs = vec![];
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                let config = Self::read_contest_list_config(&path, election_id)?;
                configs.push(config);
            }
        }

        Ok(InputElectionConfig {
            id: election_id,
            name: election.name,
            ballot_styles: election.ballot_styles,
            contest_list: configs,
            path: path.to_path_buf(),
            census: election.census,
            total_votes: election.total_votes,
        })
    }

    fn read_contest_list_config(path: &Path, election_id: Uuid) -> Result<InputContestConfig> {
        let contest_id =
            Self::parse_path_components(path, PREFIX_CONTEST).ok_or(Error::IDNotFound)?;
        let config_path_contest = path.join(CONTEST_CONFIG_FILE);
        if !config_path_contest.exists() {
            return Err(Error::ContestConfigNotFound(contest_id));
        }
        let config_file = fs::File::open(&config_path_contest)
            .map_err(|e| Error::FileAccess(config_path_contest.clone(), e))?;
        let contest: Contest = parse_file(config_file)?;

        let entries = fs::read_dir(path)?;
        let mut configs = vec![];
        for entry in entries {
            let path_area = entry?.path();
            if path_area.is_dir() {
                let area_id = Self::parse_path_components(&path_area, PREFIX_AREA)
                    .ok_or(Error::IDNotFound)?;

                let config_path_area = path
                    .join(format!("{PREFIX_AREA}{area_id}"))
                    .join(AREA_CONFIG_FILE);

                if !config_path_area.exists() {
                    return Err(Error::AreaConfigNotFound(area_id));
                }

                let config_file = fs::File::open(&config_path_area)
                    .map_err(|e| Error::FileAccess(config_path_area.clone(), e))?;
                let area_config: AreaConfig = parse_file(config_file)?;

                configs.push(InputAreaConfig {
                    id: area_id,
                    election_id,
                    contest_id,
                    census: area_config.census,
                    path: path_area,
                    area: area_config.clone(),
                });
            }
        }

        Ok(InputContestConfig {
            id: contest_id,
            election_id,
            contest,
            area_list: configs,
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
pub struct InputElectionConfig {
    pub id: Uuid,
    pub name: String,
    pub ballot_styles: Vec<BallotStyle>,
    pub contest_list: Vec<InputContestConfig>,
    pub path: PathBuf,
    pub census: u64,
    pub total_votes: u64,
}

#[derive(Debug)]
pub struct InputContestConfig {
    pub id: Uuid,
    pub election_id: Uuid,
    pub contest: Contest,
    pub area_list: Vec<InputAreaConfig>,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct InputAreaConfig {
    pub id: Uuid,
    pub election_id: Uuid,
    pub contest_id: Uuid,
    pub census: u64,
    pub path: PathBuf,
    pub area: AreaConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ElectionConfig {
    pub id: Uuid,
    pub name: String,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub census: u64,
    pub total_votes: u64,
    pub ballot_styles: Vec<BallotStyle>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AreaConfig {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub census: u64,
    pub parent_id: Option<Uuid>,
}

impl Into<TreeNodeArea> for &AreaConfig {
    fn into(self) -> TreeNodeArea {
        TreeNodeArea {
            id: self.id.to_string(),
            tenant_id: self.tenant_id.to_string(),
            election_event_id: self.election_event_id.to_string(),
            parent_id: self.parent_id.clone().map(|val| val.to_string()),
        }
    }
}
