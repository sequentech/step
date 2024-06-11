// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::tally;
use crate::pipes::{
    decode_ballots::OUTPUT_DECODED_BALLOTS_FILE,
    error::{Error, Result},
    pipe_inputs::{PipeInputs, PREFIX_TALLY_SHEET},
    pipe_name::PipeNameOutputDir,
    Pipe,
};
use crate::utils::HasId;
use sequent_core::{
    ballot::Candidate,
    services::area_tree::TreeNodeArea,
    types::hasura::core::TallySheet,
    util::path::{get_folder_name, list_subfolders},
};
use sequent_core::{ballot::Contest, services::area_tree::TreeNode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tracing::instrument;
use uuid::Uuid;

pub const OUTPUT_CONTEST_RESULT_FILE: &str = "contest_result.json";
pub const OUTPUT_CONTEST_RESULT_AGGREGATE_FOLDER: &str = "aggregate";
pub const INPUT_TALLY_SHEET_FILE: &str = "tally-sheet.json";

pub struct DoTally {
    pub pipe_inputs: PipeInputs,
}

impl DoTally {
    #[instrument(skip_all, name = "DoTally::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

pub fn list_tally_sheet_subfolders(path: &Path) -> Vec<PathBuf> {
    let subfolders = list_subfolders(&path);
    let tally_sheet_folders: Vec<PathBuf> = subfolders
        .into_iter()
        .filter(|path| {
            let Some(folder_name) = get_folder_name(path) else {
                return false;
            };
            folder_name.starts_with(PREFIX_TALLY_SHEET)
        })
        .collect();
    tally_sheet_folders
}

impl Pipe for DoTally {
    #[instrument(skip_all, name = "DoTally::new")]
    fn exec(&self) -> Result<()> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());
        let output_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DoTally.as_ref());

        let tally_sheets_dir = self.pipe_inputs.root_path_tally_sheets.clone();

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                let mut contest_ballot_files = vec![];
                let mut sum_census: u64 = 0;

                let areas: Vec<TreeNodeArea> = contest_input
                    .area_list
                    .iter()
                    .map(|area| (&area.area).into())
                    .collect();

                let areas_tree = TreeNode::<()>::from_areas(areas).map_err(|err| {
                    Error::UnexpectedError(format!("Error building area tree {:?}", err))
                })?;
                let census_map: HashMap<String, u64> = contest_input
                    .area_list
                    .iter()
                    .map(|area_input| (area_input.area.id.to_string(), area_input.census))
                    .collect();

                for area_input in &contest_input.area_list {
                    let base_input_path = PipeInputs::build_path(
                        &input_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    );

                    let base_output_path = PipeInputs::build_path(
                        &output_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    );

                    let decoded_ballots_file = base_input_path.join(OUTPUT_DECODED_BALLOTS_FILE);

                    // create aggregate tally from children areas
                    let Some(area_tree) = areas_tree.find_area(&area_input.id.to_string()) else {
                        return Err(Error::UnexpectedError(format!(
                            "Error finding area {} in areas tree {:?}",
                            area_input.id, areas_tree
                        )));
                    };
                    // Note: children areas includes itself
                    let children_areas = area_tree.get_all_children();
                    let num_children_areas = children_areas
                        .clone()
                        .iter()
                        .filter(|child| child.id != area_input.id.to_string())
                        .count();

                    if num_children_areas > 0usize {
                        let base_aggregate_path =
                            base_output_path.join(OUTPUT_CONTEST_RESULT_AGGREGATE_FOLDER);
                        fs::create_dir_all(&base_aggregate_path)?;

                        let census_size: u64 = children_areas
                            .iter()
                            .map(|child_area| census_map.get(&child_area.id))
                            .filter_map(|census| census.clone())
                            .sum();

                        let children_area_paths: Vec<PathBuf> = children_areas
                            .iter()
                            .map(|child_area| -> Result<PathBuf> {
                                Ok(PipeInputs::build_path(
                                    &input_dir,
                                    &contest_input.election_id,
                                    Some(&contest_input.id),
                                    Some(&Uuid::parse_str(&child_area.id).map_err(|err| {
                                        Error::UnexpectedError(format!("{:?}", err))
                                    })?),
                                )
                                .join(OUTPUT_DECODED_BALLOTS_FILE))
                            })
                            .collect::<Result<Vec<PathBuf>>>()?;

                        let counting_algorithm = tally::create_tally(
                            &contest_input.contest,
                            children_area_paths,
                            census_size,
                        )
                        .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                        let res: ContestResult = counting_algorithm
                            .tally()
                            .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                        let file_path = base_aggregate_path.join(OUTPUT_CONTEST_RESULT_FILE);

                        let file = fs::File::create(file_path)?;

                        serde_json::to_writer(file, &res)?;
                    }

                    // create area tally
                    let counting_algorithm = tally::create_tally(
                        &contest_input.contest,
                        vec![decoded_ballots_file.clone()],
                        area_input.census,
                    )
                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                    let res = counting_algorithm
                        .tally()
                        .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                    fs::create_dir_all(&base_output_path)?;
                    let file_path = base_output_path.join(OUTPUT_CONTEST_RESULT_FILE);

                    let file = fs::File::create(file_path)?;

                    serde_json::to_writer(file, &res)?;

                    contest_ballot_files.push(decoded_ballots_file);

                    sum_census += area_input.census;

                    // tally sheets tally
                    let input_tally_sheets_dir = PipeInputs::build_path(
                        &tally_sheets_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    );
                    if input_tally_sheets_dir.exists() && input_tally_sheets_dir.is_dir() {
                        let tally_sheet_folders =
                            list_tally_sheet_subfolders(&input_tally_sheets_dir);
                        for tally_sheet_folder in tally_sheet_folders {
                            // read tally sheet
                            let tally_sheets_file_path =
                                tally_sheet_folder.join(INPUT_TALLY_SHEET_FILE);
                            let tally_sheet_str = fs::read_to_string(&tally_sheets_file_path)
                                .map_err(|e| {
                                    Error::FileAccess(tally_sheets_file_path.to_path_buf(), e)
                                })?;
                            let tally_sheet: TallySheet = serde_json::from_str(&tally_sheet_str)?;
                            let output_tally_sheets_folder_path =
                                PipeInputs::build_tally_sheet_path(
                                    &base_output_path,
                                    &tally_sheet.id,
                                );
                            fs::create_dir_all(&output_tally_sheets_folder_path)?;
                            let contest_result =
                                tally::process_tally_sheet(&tally_sheet, &contest_input.contest)
                                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                            let output_tally_sheets_file_path =
                                output_tally_sheets_folder_path.join(OUTPUT_CONTEST_RESULT_FILE);
                            let contest_result_file =
                                fs::File::create(&output_tally_sheets_file_path)?;
                            serde_json::to_writer(contest_result_file, &contest_result)?;
                        }
                    }
                }

                // create contest tally
                let counting_algorithm =
                    tally::create_tally(&contest_input.contest, contest_ballot_files, sum_census)
                        .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                let res = counting_algorithm
                    .tally()
                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                let mut file = PipeInputs::build_path(
                    &output_dir,
                    &contest_input.election_id,
                    Some(&contest_input.id),
                    None,
                );
                file.push(OUTPUT_CONTEST_RESULT_FILE);

                let file = fs::File::create(file)?;

                serde_json::to_writer(file, &res)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidVotes {
    pub explicit: u64,
    pub implicit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestResult {
    pub contest: Contest,
    pub census: u64,
    pub percentage_census: f64,
    pub total_votes: u64,
    pub percentage_total_votes: f64,
    pub total_valid_votes: u64,
    pub percentage_total_valid_votes: f64,
    pub total_invalid_votes: u64,
    pub percentage_total_invalid_votes: f64,
    pub total_blank_votes: u64,
    pub percentage_total_blank_votes: f64,
    pub invalid_votes: InvalidVotes,
    pub percentage_invalid_votes_explicit: f64,
    pub percentage_invalid_votes_implicit: f64,
    pub candidate_result: Vec<CandidateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateResult {
    pub candidate: Candidate,
    pub percentage_votes: f64,
    pub total_count: u64,
}

impl HasId for Contest {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Candidate {
    fn id(&self) -> &str {
        &self.id
    }
}
