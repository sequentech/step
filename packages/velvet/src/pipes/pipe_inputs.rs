// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::error::{Error, Result};
use crate::{
    cli::{state::Stage, CliRun},
    utils::parse_file,
};
use sequent_core::{
    ballot::{BallotStyle, Contest, ElectionPresentation, ReportDates, StringifiedPeriodDates},
    services::area_tree::TreeNodeArea,
    types::participation::VotesByChannel,
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
pub const PREFIX_ALL_AREAS: &str = "all_areas";

pub const DEFAULT_DIR_CONFIGS: &str = "default/configs";
pub const DEFAULT_DIR_BALLOTS: &str = "default/ballots";
pub const DEFAULT_DIR_TALLY_SHEETS: &str = "default/tally_sheets";
pub const DEFAULT_DIR_DATABASE: &str = "default/database";

pub const ELECTION_CONFIG_FILE: &str = "election-config.json";
pub const CONTEST_CONFIG_FILE: &str = "contest-config.json";
pub const AREA_CONFIG_FILE: &str = "area-config.json";
pub const BALLOTS_FILE: &str = "ballots.csv";
/// Placed between a ballots file's stem and extension to name the multiplier
/// of the batch it holds: every ballot in `ballots__x4.csv` counts four times.
pub const BATCH_MULTIPLIER_INFIX: &str = "__x";
const UUID_LEN: usize = 36;

/// The file holding the ballots of an area that each count `multiplier` times.
///
/// A ballot counts once unless the area's ballots were split into weight
/// batches, so multiplier 1 is the file every pipe has always used, and only
/// the other batches get a name of their own, e.g. `ballots__x4.csv`.
pub fn batch_file_name(file_name: &str, multiplier: u64) -> String {
    if multiplier == 1 {
        return file_name.to_string();
    }
    match file_name.rsplit_once('.') {
        Some((stem, extension)) => {
            format!("{stem}{BATCH_MULTIPLIER_INFIX}{multiplier}.{extension}")
        }
        None => format!("{file_name}{BATCH_MULTIPLIER_INFIX}{multiplier}"),
    }
}

/// The multiplier of the batch `candidate` holds, if it is `file_name` or one
/// of its batch files. A name shaped like a batch file whose multiplier is not
/// written the way `batch_file_name` writes it is an error rather than another
/// file, so that no ballots are silently left out of a count.
pub fn parse_batch_file_name(file_name: &str, candidate: &str) -> Result<Option<u64>> {
    if candidate == file_name {
        return Ok(Some(1));
    }
    let (stem, extension) = match file_name.rsplit_once('.') {
        Some((stem, extension)) => (stem, format!(".{extension}")),
        None => (file_name, String::new()),
    };
    let Some(multiplier) = candidate
        .strip_prefix(stem)
        .and_then(|rest| rest.strip_prefix(BATCH_MULTIPLIER_INFIX))
        .and_then(|rest| rest.strip_suffix(extension.as_str()))
    else {
        return Ok(None);
    };
    match multiplier.parse::<u64>() {
        Ok(value) if value > 1 && value.to_string() == multiplier => Ok(Some(value)),
        _ => Err(Error::UnexpectedError(format!(
            "Invalid batch multiplier {multiplier:?} in ballots file {candidate:?}"
        ))),
    }
}

/// Every batch file of `file_name` in `dir`, the plain one included, with the
/// multiplier of each, ordered by multiplier. A missing directory has none.
pub fn list_batch_files(dir: &Path, file_name: &str) -> Result<Vec<(PathBuf, u64)>> {
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut files = vec![];
    for entry in fs::read_dir(dir).map_err(|e| Error::FileAccess(dir.to_path_buf(), e))? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if let Some(multiplier) = parse_batch_file_name(file_name, name)? {
            files.push((path, multiplier));
        }
    }
    files.sort_by_key(|(_, multiplier)| *multiplier);
    Ok(files)
}

/// Refuses ballot-by-ballot output for an area whose ballots were split into
/// weight batches. A ballot there stands for several, so showing it once would
/// misstate what was counted, and showing it once per batch would spell out
/// its voter's weight.
pub fn ensure_unbatched(dir: &Path, file_name: &str) -> Result<()> {
    match list_batch_files(dir, file_name)?
        .into_iter()
        .find(|(_, multiplier)| *multiplier != 1)
    {
        Some((path, multiplier)) => Err(Error::UnexpectedError(format!(
            "{} holds ballots that each count {multiplier} times. Ballot images are not \
             available for ballots split into vote weight batches",
            path.display()
        ))),
        None => Ok(()),
    }
}

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
    pub fn build_consolidated_report_path(root: &Path, election_id: &Uuid) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(root);
        path.push(format!("{}{}", PREFIX_ELECTION, election_id));
        path.push(format!("{}", PREFIX_ALL_AREAS));

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
            presentation: election.presentation,
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
                // Folder names are external input. Check the byte length before
                // subtraction, then use get() so a UTF-8 boundary cannot panic.
                if res.len() >= UUID_LEN {
                    return res
                        .get(res.len() - UUID_LEN..)
                        .and_then(|suffix| Uuid::parse_str(suffix).ok());
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
    pub presentation: Option<ElectionPresentation>,
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
    pub presentation: Option<ElectionPresentation>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub votes_by_channel: Option<VotesByChannel>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn a_batch_counted_once_keeps_the_plain_file_name() {
        assert_eq!(batch_file_name(BALLOTS_FILE, 1), BALLOTS_FILE);
        assert_eq!(batch_file_name(BALLOTS_FILE, 4), "ballots__x4.csv");
        assert_eq!(
            batch_file_name("decoded_ballots.json", 2_147_483_648),
            "decoded_ballots__x2147483648.json"
        );
        assert_eq!(batch_file_name("ballots", 2), "ballots__x2");
    }

    #[test]
    fn batch_file_names_parse_back_to_their_multiplier() {
        for multiplier in [1u64, 2, 4, 65_536, 2_147_483_648, u64::MAX] {
            let name = batch_file_name(BALLOTS_FILE, multiplier);
            assert_eq!(
                parse_batch_file_name(BALLOTS_FILE, &name).unwrap(),
                Some(multiplier),
                "{name}"
            );
        }
    }

    #[test]
    fn unrelated_files_are_not_batches() {
        for name in [
            "area-config.json",
            "ballots.csv.bak",
            "decoded_ballots.json",
            "ballots_x4.csv",
            "ballots__x4.json",
        ] {
            assert_eq!(
                parse_batch_file_name(BALLOTS_FILE, name).unwrap(),
                None,
                "{name}"
            );
        }
    }

    #[test]
    fn a_malformed_multiplier_is_an_error_not_a_skipped_file() {
        // Skipping it would leave its ballots out of the count.
        for name in [
            "ballots__x.csv",
            "ballots__x1.csv",
            "ballots__x0.csv",
            "ballots__x04.csv",
            "ballots__x+4.csv",
            "ballots__x4a.csv",
            "ballots__x18446744073709551616.csv",
        ] {
            assert!(
                parse_batch_file_name(BALLOTS_FILE, name).is_err(),
                "{name} should be refused"
            );
        }
    }

    #[test]
    fn ballot_by_ballot_output_is_refused_only_for_multiplied_batches() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(BALLOTS_FILE), "").unwrap();
        assert!(ensure_unbatched(dir.path(), BALLOTS_FILE).is_ok());
        assert!(ensure_unbatched(&dir.path().join("missing"), BALLOTS_FILE).is_ok());

        fs::write(dir.path().join("ballots__x2.csv"), "").unwrap();
        assert!(ensure_unbatched(dir.path(), BALLOTS_FILE).is_err());
    }

    #[test]
    fn batch_files_are_listed_with_their_multipliers_in_order() {
        let dir = tempdir().unwrap();
        for name in [
            "ballots__x8.csv",
            BALLOTS_FILE,
            "ballots__x2.csv",
            "area-config.json",
        ] {
            fs::write(dir.path().join(name), "").unwrap();
        }
        fs::create_dir(dir.path().join("ballots__x16.csv")).unwrap();

        let files = list_batch_files(dir.path(), BALLOTS_FILE).unwrap();

        assert_eq!(
            files,
            vec![
                (dir.path().join(BALLOTS_FILE), 1),
                (dir.path().join("ballots__x2.csv"), 2),
                (dir.path().join("ballots__x8.csv"), 8),
            ]
        );
        assert_eq!(
            list_batch_files(&dir.path().join("missing"), BALLOTS_FILE).unwrap(),
            vec![]
        );
    }
}
