// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::error::{Error, Result};
use crate::{
    cli::{state::Stage, CliRun},
    utils::parse_file,
};
use sequent_core::{
    ballot::{BallotStyle, Contest, ReportDates, StringifiedPeriodDates},
    services::area_tree::TreeNodeArea,
    util::path::get_folder_name,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tracing::{info, instrument};
use uuid::Uuid;

pub const PREFIX_ELECTION: &str = "election__";
pub const PREFIX_CONTEST: &str = "contest__";
pub const PREFIX_AREA: &str = "area__";
pub const PREFIX_TALLY_SHEET: &str = "tally_sheet__";

pub const DEFAULT_DIR_CONFIGS: &str = "default/configs";
pub const DEFAULT_DIR_BALLOTS: &str = "default/ballots";
pub const DEFAULT_DIR_TALLY_SHEETS: &str = "default/tally_sheets";
pub const DEFAULT_DIR_DATABASE: &str = "default/database";

pub const ELECTION_CONFIG_FILE: &str = "election-config.json";
pub const CONTEST_CONFIG_FILE: &str = "contest-config.json";
pub const AREA_CONFIG_FILE: &str = "area-config.json";
pub const BALLOTS_FILE: &str = "ballots.csv";
const UUID_LEN: usize = 36;

#[derive(Debug)]
pub struct PipeInputs {
    pub cli: CliRun,
    pub root_path_config: PathBuf,
    pub root_path_ballots: PathBuf,
    pub root_path_tally_sheets: PathBuf,
    pub root_path_database: PathBuf,
    pub stage: Stage,
    pub election_list: Vec<InputElectionConfig>,
}

impl PipeInputs {
    #[instrument(err, skip_all, name = "PipeInputs::new")]
    pub fn new(cli: CliRun, stage: Stage) -> Result<Self> {
        let root_path_config = &cli.input_dir.join(DEFAULT_DIR_CONFIGS);
        let root_path_ballots = &cli.input_dir.join(DEFAULT_DIR_BALLOTS);
        let root_path_tally_sheets = &cli.input_dir.join(DEFAULT_DIR_TALLY_SHEETS);
        let root_path_database = &cli.input_dir.join(DEFAULT_DIR_DATABASE);

        let election_list = Self::read_input_dir_config(root_path_config.as_path())?;
        Ok(Self {
            cli,
            root_path_config: root_path_config.to_path_buf(),
            root_path_ballots: root_path_ballots.to_path_buf(),
            root_path_tally_sheets: root_path_tally_sheets.to_path_buf(),
            root_path_database: root_path_database.to_path_buf(),
            stage,
            election_list,
        })
    }

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
    pub fn build_path_by_area(
        root: &Path,
        election_id: &Uuid,
        contest_id: Option<&Uuid>,
        area_id: Option<&Uuid>,
    ) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(root);
        path.push(format!("{}{}", PREFIX_ELECTION, election_id));

        if let Some(area_id) = area_id {
            path.push(format!("{}{}", PREFIX_AREA, area_id));
        }

        if let Some(contest_id) = contest_id {
            path.push(format!("{}{}", PREFIX_CONTEST, contest_id));
        }

        path
    }

    /// Returns the path at which multi contest ballots are present,
    /// relative to some supplied root path.
    ///
    /// This path is used both to find input ballots and to output decoded
    /// ballots.
    ///
    #[instrument(skip_all)]
    pub fn mcballots_path(root: &Path, election_id: &Uuid, area_id: &Uuid) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(root);
        path.push(format!("{}{}", PREFIX_ELECTION, election_id));
        path.push(format!("{}{}", PREFIX_AREA, area_id));

        path
    }

    #[instrument(skip_all)]
    pub fn build_tally_sheet_path(root: &Path, tally_sheet_id: &str) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(root);
        path.push(format!("{}{}", PREFIX_TALLY_SHEET, tally_sheet_id));
        path
    }

    #[instrument(skip_all)]
    pub fn get_tally_sheet_id_from_path(path: &Path) -> Option<String> {
        let Some(folder_name) = get_folder_name(path) else {
            return None;
        };
        if folder_name.starts_with(PREFIX_TALLY_SHEET) {
            folder_name
                .strip_prefix(PREFIX_TALLY_SHEET)
                .map(|val| val.to_string())
        } else {
            None
        }
    }

    #[instrument(err)]
    fn read_input_dir_config(input_dir: &Path) -> Result<Vec<InputElectionConfig>> {
        let entries = fs::read_dir(input_dir)?;

        let mut configs = vec![];
        for entry in entries {
            let config = Self::read_election_list_config(&entry?.path())?;
            configs.push(config);
        }

        Ok(configs)
    }

    #[instrument(err)]
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
            alias: election.alias,
            description: election.description,
            annotations: election.annotations,
            election_event_annotations: election.election_event_annotations,
            dates: election.dates,
            ballot_styles: election.ballot_styles,
            contest_list: configs,
            path: path.to_path_buf(),
            census: election.census,
            total_votes: election.total_votes,
            areas: election.areas,
        })
    }

    #[instrument(err)]
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
                    auditable_votes: area_config.auditable_votes,
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

    #[instrument]
    fn parse_path_components(path: &Path, prefix: &str) -> Option<Uuid> {
        for component in path.components() {
            let part = component.as_os_str().to_string_lossy();

            if let Some(res) = part.strip_prefix(prefix) {
                let slice = &res[res.len() - UUID_LEN..];
                // Check if the string length is at least 36
                if res.len() >= UUID_LEN {
                    // Use the last 36 characters for UUID parsing
                    return Uuid::parse_str(slice).ok();
                }
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct InputElectionConfig {
    pub id: Uuid,
    pub name: String,
    pub alias: String,
    pub description: String,
    pub dates: Option<StringifiedPeriodDates>,
    pub annotations: HashMap<String, String>,
    pub election_event_annotations: HashMap<String, String>,
    pub ballot_styles: Vec<BallotStyle>,
    pub contest_list: Vec<InputContestConfig>,
    pub path: PathBuf,
    pub census: u64,
    pub total_votes: u64,
    pub areas: Vec<TreeNodeArea>,
}

#[derive(Debug, Clone)]
pub struct AreaContest {
    pub area_name: String,
    pub contests: Vec<Contest>,
}

impl InputElectionConfig {
    #[instrument(skip_all)]
    pub(crate) fn get_area_contest_map(&self) -> HashMap<Uuid, AreaContest> {
        let mut ret: HashMap<Uuid, AreaContest> = HashMap::new();

        for contest_input in &self.contest_list {
            for area_input in &contest_input.area_list {
                let key = area_input.id;
                let value = contest_input.contest.clone();
                let area_name = area_input.area.name.clone();
                if let Some(area_contests) = ret.get_mut(&key) {
                    area_contests.contests.push(value);
                } else {
                    ret.insert(
                        key,
                        AreaContest {
                            area_name,
                            contests: vec![value],
                        },
                    );
                }
            }
        }

        ret
    }
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
    pub auditable_votes: u64,
    pub path: PathBuf,
    pub area: AreaConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ElectionConfig {
    pub id: Uuid,
    pub name: String,
    pub alias: String,
    pub description: String,
    pub annotations: HashMap<String, String>,
    pub election_event_annotations: HashMap<String, String>,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub census: u64,
    pub total_votes: u64,
    pub ballot_styles: Vec<BallotStyle>,
    pub areas: Vec<TreeNodeArea>,
    pub dates: Option<StringifiedPeriodDates>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AreaConfig {
    pub id: Uuid,
    pub name: String,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub census: u64,
    pub parent_id: Option<Uuid>,
    pub auditable_votes: u64,
}

impl Into<TreeNodeArea> for &AreaConfig {
    fn into(self) -> TreeNodeArea {
        TreeNodeArea {
            id: self.id.to_string(),
            tenant_id: self.tenant_id.to_string(),
            annotations: Default::default(),
            election_event_id: self.election_event_id.to_string(),
            parent_id: self.parent_id.clone().map(|val| val.to_string()),
        }
    }
}
