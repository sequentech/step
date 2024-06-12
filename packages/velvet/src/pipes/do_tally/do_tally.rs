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
use std::cmp;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tracing::instrument;
use uuid::Uuid;

pub const OUTPUT_CONTEST_RESULT_FILE: &str = "contest_result.json";
pub const OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER: &str = "aggregate";
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

                let mut tally_sheet_results: Vec<ContestResult> = vec![];

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

                    if num_children_areas > 1usize {
                        let base_aggregate_path = base_output_path
                            .join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
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
                            vec![],
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
                        vec![],
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

                            tally_sheet_results.push(contest_result);
                        }
                    }
                }

                // create contest tally
                let counting_algorithm = tally::create_tally(
                    &contest_input.contest,
                    contest_ballot_files,
                    sum_census,
                    tally_sheet_results.clone(),
                )
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

impl InvalidVotes {
    pub fn aggregate(&self, other: &InvalidVotes) -> InvalidVotes {
        let mut sum = self.clone();

        sum.explicit += other.explicit;
        sum.implicit += other.implicit;
        sum
    }
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

impl ContestResult {
    pub fn calculate_percentages(&self) -> ContestResult {
        let valid_not_blank = self.total_valid_votes - self.total_blank_votes;
        let candidate_result: Vec<CandidateResult> = self
            .candidate_result
            .clone()
            .into_iter()
            .map(|candidate_result| {
                let percentage_votes = (candidate_result.total_count as f64
                    / cmp::max(1, valid_not_blank) as f64)
                    * 100.0;
                let mut new_candidate_result = candidate_result.clone();
                new_candidate_result.percentage_votes = percentage_votes;

                new_candidate_result
            })
            .collect();
        let total_votes = self.total_votes;
        let total_votes_base = cmp::max(1, total_votes) as f64;
        let count_valid = self.total_valid_votes;

        let census_base = cmp::max(1, self.census) as f64;
        let percentage_total_votes = (total_votes as f64) * 100.0 / census_base;
        let percentage_total_valid_votes = (count_valid as f64 * 100.0) / total_votes_base;
        let percentage_total_invalid_votes =
            (self.total_invalid_votes as f64 * 100.0) / total_votes_base;
        let percentage_total_blank_votes =
            (self.total_blank_votes as f64 * 100.0) / total_votes_base;
        let percentage_invalid_votes_explicit =
            (self.invalid_votes.explicit as f64 * 100.0) / total_votes_base;
        let percentage_invalid_votes_implicit =
            (self.invalid_votes.implicit as f64 * 100.0) / total_votes_base;

        let mut contest_result = self.clone();
        contest_result.percentage_census = 100.0;
        contest_result.percentage_total_votes = percentage_total_votes.clamp(0.0, 100.0);
        contest_result.percentage_total_valid_votes =
            percentage_total_valid_votes.clamp(0.0, 100.0);
        contest_result.percentage_total_invalid_votes =
            percentage_total_invalid_votes.clamp(0.0, 100.0);
        contest_result.percentage_total_blank_votes =
            percentage_total_blank_votes.clamp(0.0, 100.0);
        contest_result.percentage_invalid_votes_explicit =
            percentage_invalid_votes_explicit.clamp(0.0, 100.0);
        contest_result.percentage_invalid_votes_implicit =
            percentage_invalid_votes_implicit.clamp(0.0, 100.0);
        contest_result.candidate_result = candidate_result;
        contest_result
    }

    pub fn aggregate(&self, other: &ContestResult) -> ContestResult {
        let mut aggregate = self.clone();
        aggregate.census += other.census;
        aggregate.total_votes += other.total_votes;
        aggregate.total_valid_votes += other.total_valid_votes;
        aggregate.total_invalid_votes += other.total_invalid_votes;
        aggregate.total_blank_votes += other.total_blank_votes;
        aggregate.total_blank_votes += other.total_blank_votes;
        aggregate.invalid_votes = aggregate.invalid_votes.aggregate(&other.invalid_votes);
        let one_map: HashMap<String, CandidateResult> = self
            .candidate_result
            .iter()
            .map(|candidate_result| {
                (
                    candidate_result.candidate.id.clone(),
                    candidate_result.clone(),
                )
            })
            .collect();
        let other_map: HashMap<String, CandidateResult> = other
            .candidate_result
            .iter()
            .map(|candidate_result| {
                (
                    candidate_result.candidate.id.clone(),
                    candidate_result.clone(),
                )
            })
            .collect();
        let mut candidate_ids: HashSet<String> = HashSet::new();
        candidate_ids.extend(one_map.clone().into_keys().collect::<Vec<String>>());
        candidate_ids.extend(other_map.clone().into_keys().collect::<Vec<String>>());
        aggregate.candidate_result = vec![];
        for candidate_id in candidate_ids {
            let one_opt = one_map.get(&candidate_id);
            let other_opt = other_map.get(&candidate_id);
            if one_opt.is_some() && other_opt.is_some() {
                if let Some(one) = one_opt {
                    if let Some(other) = other_opt {
                        let mut new_candidate = one.clone();
                        new_candidate.total_count += other.total_count;
                    } else {
                        aggregate.candidate_result.push(one.clone());
                    }
                }
            } else {
                if let Some(one) = one_opt {
                    aggregate.candidate_result.push(one.clone());
                } else if let Some(other) = other_opt {
                    aggregate.candidate_result.push(other.clone());
                }
            }
        }

        aggregate.calculate_percentages()
    }
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
