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
use rayon::prelude::*;
use sequent_core::{
    ballot::Candidate,
    services::area_tree::TreeNodeArea,
    types::{hasura::core::TallySheet, tally_sheets::VotingChannel},
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
use tracing::{event, info, instrument, Level};
use uuid::Uuid;
use std::sync::Arc;

pub const OUTPUT_CONTEST_RESULT_FILE: &str = "contest_result.json";
pub const OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER: &str = "aggregate";
pub const INPUT_TALLY_SHEET_FILE: &str = "tally-sheet.json";
pub const OUTPUT_BREAKDOWNS_FOLDER: &str = "breakdowns";

pub struct DoTally {
    pub pipe_inputs: PipeInputs,
}

impl DoTally {
    #[instrument(skip_all, name = "DoTally::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

#[instrument]
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

impl DoTally {
    #[instrument(err, skip_all)]
    fn save_tally_sheets_breakdown(
        &self,
        tally_sheet_results: &Vec<(ContestResult, TallySheet)>,
        base_file_path: &PathBuf,
    ) -> Result<()> {
        let base_breakdown_path = base_file_path.join(OUTPUT_BREAKDOWNS_FOLDER);
        let mut breakdown_map: HashMap<VotingChannel, ContestResult> = HashMap::new();

        for (contest_result, tally_sheet) in tally_sheet_results {
            let channel: VotingChannel = tally_sheet.channel.clone().into();

            breakdown_map
                .entry(channel)
                .and_modify(|current_result| {
                    current_result.aggregate(&contest_result, true);
                })
                .or_insert_with(|| contest_result.clone());
        }

        for (channel, contest_result) in breakdown_map {
            let breakdown_folder_path = base_breakdown_path.join(&channel.to_string());
            fs::create_dir_all(&breakdown_folder_path)?;
            let breakdown_file_path = breakdown_folder_path.join((OUTPUT_CONTEST_RESULT_FILE));
            let contest_result_file = fs::File::create(&breakdown_file_path)?;
            serde_json::to_writer(contest_result_file, &contest_result)?;
        }

        Ok(())
    }
}

impl Pipe for DoTally {
    #[instrument(err, skip_all, name = "DoTally::exec")]
    fn exec(&self) -> Result<()> {
        let input_dir_base = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());
        let output_dir_base = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DoTally.as_ref());
        let tally_sheets_dir_base = self.pipe_inputs.root_path_tally_sheets.clone();

        for election_input in &self.pipe_inputs.election_list {
            // Parallelize the processing of each contest
            election_input
                .contest_list
                .par_iter()
                .map(|contest_input| {
                    let input_dir = input_dir_base.clone();
                    let output_dir = output_dir_base.clone();
                    let tally_sheets_dir = tally_sheets_dir_base.clone();

                    // These are specific to the contest and need to be cloned for use in area processing.
                    let election_id_for_contest = contest_input.election_id.clone();
                    let contest_id_for_contest = contest_input.id.clone();
                    let contest_object_for_contest = contest_input.contest.clone();

                    // --- Start of logic for a single contest ---
                    let _areas_info: Vec<TreeNodeArea> = contest_input // Renamed, original `areas` was unused after info
                        .area_list
                        .iter()
                        .map(|area| (&area.area).into())
                        .collect();
                    info!(
                        "areas for contest {}: {:?}",
                        contest_id_for_contest, _areas_info
                    );

                    let areas_tree = Arc::new(TreeNode::<()>::from_areas(election_input.areas.clone())
                        .map_err(|err| {
                            Error::UnexpectedError(format!(
                                "Error building area tree for contest {}: {:?}",
                                contest_id_for_contest, err
                            ))
                        })?);

                    let census_map: HashMap<String, u64> = contest_input
                        .area_list
                        .iter()
                        .map(|area_input| (area_input.area.id.to_string(), area_input.census))
                        .collect();
                    let auditable_votes_map: HashMap<String, u64> = contest_input
                        .area_list
                        .iter()
                        .map(|area_input| {
                            (area_input.area.id.to_string(), area_input.auditable_votes)
                        })
                        .collect();

                    // Parallelize processing for each area within this contest
                    let area_processing_results: Result<Vec<_>, Error> = contest_input
                        .area_list
                        .par_iter()
                        .map(|area_input| {
                            // Clone data needed per area task.
                            let area_id = area_input.id.clone();
                            let election_id = election_id_for_contest.clone();
                            let contest_id = contest_id_for_contest.clone();
                            let contest_object = contest_object_for_contest.clone();

                            let base_input_path = PipeInputs::build_path(
                                &input_dir,
                                &election_id,
                                Some(&contest_id),
                                Some(&area_id),
                            );

                            let base_output_path = PipeInputs::build_path(
                                &output_dir,
                                &election_id,
                                Some(&contest_id),
                                Some(&area_id),
                            );

                            let decoded_ballots_file =
                                base_input_path.join(OUTPUT_DECODED_BALLOTS_FILE);

                            // Create aggregate tally from children areas
                            let Some(area_tree_node) =
                                areas_tree.as_ref().find_area(&area_input.id.to_string())
                            else {
                                return Err(Error::UnexpectedError(format!(
                                    "Error finding area {} in areas tree for contest {}",
                                    area_input.id, contest_id
                                )));
                            };
                            let children_areas = area_tree_node.get_all_children();
                            let num_children_areas = children_areas
                                .iter()
                                .filter(|child| child.id != area_input.id.to_string())
                                .count();

                            if num_children_areas > 0usize {
                                let base_aggregate_path = base_output_path
                                    .join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
                                fs::create_dir_all(&base_aggregate_path)?;

                                let census_size: u64 = children_areas
                                    .iter()
                                    .filter_map(|child_area| {
                                        census_map.get(&child_area.id).copied()
                                    })
                                    .sum();
                                let auditable_votes_size: u64 = children_areas
                                    .iter()
                                    .filter_map(|child_area| {
                                        auditable_votes_map.get(&child_area.id).copied()
                                    })
                                    .sum();

                                let children_area_paths: Vec<PathBuf> = children_areas
                                    .iter()
                                    .map(|child_area| -> Result<PathBuf, Error> {
                                        Ok(PipeInputs::build_path(
                                            &input_dir,
                                            &election_id,
                                            Some(&contest_id),
                                            Some(&Uuid::parse_str(&child_area.id).map_err(
                                                |err| {
                                                    Error::UnexpectedError(format!(
                                                        "Uuid parse error: {err:?}"
                                                    ))
                                                },
                                            )?),
                                        )
                                        .join(OUTPUT_DECODED_BALLOTS_FILE))
                                    })
                                    .collect::<Result<Vec<PathBuf>, Error>>()?;

                                let counting_algorithm = tally::create_tally(
                                    &contest_object,
                                    children_area_paths,
                                    census_size,
                                    auditable_votes_size,
                                    vec![],
                                )
                                .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                                let res: ContestResult = counting_algorithm
                                    .tally()
                                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                                let file_path =
                                    base_aggregate_path.join(OUTPUT_CONTEST_RESULT_FILE);
                                let file = fs::File::create(file_path)?;
                                serde_json::to_writer_pretty(file, &res)?; // Using pretty for readability
                            }

                            // Create area tally
                            let counting_algorithm_area = tally::create_tally(
                                &contest_object,
                                vec![decoded_ballots_file.clone()],
                                area_input.census,
                                area_input.auditable_votes,
                                vec![],
                            )
                            .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                            let res_area = counting_algorithm_area
                                .tally()
                                .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                            fs::create_dir_all(&base_output_path)?;
                            let file_path_area = base_output_path.join(OUTPUT_CONTEST_RESULT_FILE);
                            let file_area = fs::File::create(file_path_area)?;
                            serde_json::to_writer_pretty(file_area, &res_area)?; // Using pretty

                            // Tally sheets tally for this area
                            let mut area_specific_tally_sheet_results: Vec<(
                                ContestResult,
                                TallySheet,
                            )> = vec![];
                            let input_tally_sheets_dir_path = PipeInputs::build_path(
                                &tally_sheets_dir,
                                &election_id,
                                Some(&contest_id),
                                Some(&area_id),
                            );

                            if input_tally_sheets_dir_path.exists()
                                && input_tally_sheets_dir_path.is_dir()
                            {
                                let tally_sheet_folders =
                                    list_tally_sheet_subfolders(&input_tally_sheets_dir_path);
                                for tally_sheet_folder in tally_sheet_folders {
                                    let tally_sheets_file_path =
                                        tally_sheet_folder.join(INPUT_TALLY_SHEET_FILE);
                                    let tally_sheet_str = fs::read_to_string(
                                        &tally_sheets_file_path,
                                    )
                                    .map_err(|e| {
                                        Error::FileAccess(tally_sheets_file_path.to_path_buf(), e)
                                    })?;
                                    let tally_sheet: TallySheet =
                                        serde_json::from_str(&tally_sheet_str)?;
                                    let output_tally_sheets_folder_path =
                                        PipeInputs::build_tally_sheet_path(
                                            &base_output_path,
                                            &tally_sheet.id, // Assuming TallySheet has an id field
                                        );
                                    fs::create_dir_all(&output_tally_sheets_folder_path)?;
                                    let contest_result_sheet =
                                        tally::process_tally_sheet(&tally_sheet, &contest_object)
                                            .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                                    let output_tally_sheets_file_path =
                                        output_tally_sheets_folder_path
                                            .join(OUTPUT_CONTEST_RESULT_FILE);
                                    let contest_result_file_sheet =
                                        fs::File::create(&output_tally_sheets_file_path)?;
                                    serde_json::to_writer_pretty(
                                        contest_result_file_sheet,
                                        &contest_result_sheet,
                                    )?; // Using pretty
                                    area_specific_tally_sheet_results
                                        .push((contest_result_sheet, tally_sheet));
                                }
                            }
                            // Return data needed for final aggregation for the contest
                            Ok((
                                decoded_ballots_file,
                                area_input.census,
                                area_input.auditable_votes,
                                area_specific_tally_sheet_results,
                            ))
                        })
                        .collect(); // Collects Result<Vec<(PathBuf, u64, u64, Vec<_>)>, Error>

                    let collected_area_outputs = area_processing_results?; // Propagate error if any area failed

                    // Aggregate results from parallel area processing
                    let mut contest_ballot_files: Vec<PathBuf> = vec![];
                    let mut sum_census: u64 = 0;
                    let mut sum_auditable_votes: u64 = 0;
                    let mut tally_sheet_results_for_contest: Vec<(ContestResult, TallySheet)> =
                        vec![];

                    for (ballot_file, census, auditable_votes_val, sheet_results) in
                        collected_area_outputs
                    {
                        contest_ballot_files.push(ballot_file);
                        sum_census += census;
                        sum_auditable_votes += auditable_votes_val;
                        tally_sheet_results_for_contest.extend(sheet_results);
                    }

                    // Create contest-level output path (directory for the contest)
                    let contest_output_dir_path = PipeInputs::build_path(
                        &output_dir, // This is the output_dir cloned for this contest task
                        &election_id_for_contest,
                        Some(&contest_id_for_contest),
                        None, // No area_id for contest-level summary
                    );
                    fs::create_dir_all(&contest_output_dir_path)?; // Ensure contest directory exists

                    self.save_tally_sheets_breakdown(
                        &tally_sheet_results_for_contest,
                        &contest_output_dir_path,
                    )?;

                    let final_only_sheet_results: Vec<ContestResult> =
                        tally_sheet_results_for_contest
                            .iter()
                            .map(|(res, _)| res.clone())
                            .collect();

                    // Create final contest tally
                    let final_counting_algorithm = tally::create_tally(
                        &contest_object_for_contest,
                        contest_ballot_files,
                        sum_census,
                        sum_auditable_votes,
                        final_only_sheet_results,
                    )
                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                    let final_res = final_counting_algorithm
                        .tally()
                        .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                    let final_contest_result_file_path =
                        contest_output_dir_path.join(OUTPUT_CONTEST_RESULT_FILE);
                    let final_file = fs::File::create(final_contest_result_file_path)?;
                    serde_json::to_writer_pretty(final_file, &final_res)?; // Using pretty

                    Ok(()) // Result for this contest's processing
                })
                .collect::<Result<Vec<()>, Error>>()?; // Collect results from parallel contest processing
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
    #[instrument]
    pub fn aggregate(&self, other: &InvalidVotes) -> InvalidVotes {
        let mut sum = self.clone();

        sum.explicit += other.explicit;
        sum.implicit += other.implicit;
        sum
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedMetricsContest {
    // Voted more candidates than the allowed amount per contest
    pub over_votes: u64,
    // Voted less than the number of votes allowed for each contest.
    pub under_votes: u64,
    // Total actual marks count of candidates in the contest. Only counted UV and fully votes.
    pub votes_actually: u64,
    // Total expected marks for candidates if all votes were normal
    // (no under-votes, no over-votes) (valid-ballots X number of
    // votes possible in the contest)
    pub expected_votes: u64,
    //Total counted ballots
    pub total_ballots: u64,
}

impl ExtendedMetricsContest {
    #[instrument(skip_all)]
    pub fn aggregate(&self, other: &ExtendedMetricsContest) -> ExtendedMetricsContest {
        let mut result = self.clone();
        result.over_votes += other.over_votes;
        result.under_votes += other.under_votes;
        result.votes_actually += other.votes_actually;
        result.expected_votes += other.expected_votes;
        result.total_ballots += other.total_ballots;

        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedMetricsElection {
    // Number of valid ballots processed by the ACM without any
    // single mark on all contests.
    pub abstentions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestResult {
    pub contest: Contest,
    pub census: u64,
    pub percentage_census: f64,
    pub auditable_votes: u64,
    pub percentage_auditable_votes: f64,
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
    pub extended_metrics: Option<ExtendedMetricsContest>,
}

impl ContestResult {
    #[instrument(skip_all)]
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

        // `percentage_auditable_votes` is calculated over `census_base`.
        // Otherwise we could end up with strange percentages. Imagine a test
        // election with 2 auditable votes and 1 valid vote. That's maybe 66%
        // auditable votes over the census, but 200% over total votes.
        let percentage_auditable_votes = (self.auditable_votes as f64) * 100.0 / census_base;
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
        contest_result.percentage_auditable_votes = percentage_auditable_votes.clamp(0.0, 100.0);
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

    #[instrument(skip_all)]
    pub fn aggregate(&self, other: &ContestResult, add_census: bool) -> ContestResult {
        let mut aggregate = self.clone();
        if add_census {
            aggregate.census += other.census;
        }
        let aggregate_metrics = aggregate.extended_metrics.unwrap_or_default();
        aggregate_metrics.aggregate(&other.extended_metrics.clone().unwrap_or_default());
        aggregate.extended_metrics = Some(aggregate_metrics);
        aggregate.total_votes += other.total_votes;
        aggregate.total_valid_votes += other.total_valid_votes;
        aggregate.total_invalid_votes += other.total_invalid_votes;
        aggregate.total_blank_votes += other.total_blank_votes;
        aggregate.invalid_votes = aggregate.invalid_votes.aggregate(&other.invalid_votes);

        let mut candidate_map: HashMap<String, CandidateResult> = HashMap::new();

        for candidate_result in &self.candidate_result {
            candidate_map.insert(
                candidate_result.candidate.id.clone(),
                candidate_result.clone(),
            );
        }

        for candidate_result in &other.candidate_result {
            candidate_map
                .entry(candidate_result.candidate.id.clone())
                .and_modify(|entry| entry.total_count += candidate_result.total_count)
                .or_insert_with(|| candidate_result.clone());
        }

        aggregate.candidate_result = candidate_map.into_values().collect();

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
