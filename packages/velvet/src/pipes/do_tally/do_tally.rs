// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::tally;
use crate::pipes::{
    decode_ballots::OUTPUT_DECODED_BALLOTS_FILE,
    do_tally::counting_algorithm::utils::{
        get_area_tally_operation, get_area_weight, get_contest_tally_operation,
    },
    error::{Error, Result},
    pipe_inputs::{PipeInputs, PREFIX_TALLY_SHEET},
    pipe_name::PipeNameOutputDir,
    Pipe,
};

use crate::utils::HasId;
use rayon::prelude::*;
use sequent_core::{
    ballot::{BallotStyle, Candidate},
    ballot_style,
    services::area_tree::TreeNodeArea,
    sqlite::election_event,
    types::ceremonies::{ScopeOperation, TallyOperation},
    types::hasura::core::TallySheet,
    types::tally_sheets::VotingChannel,
    util::path::{get_folder_name, list_subfolders},
};
use sequent_core::{
    ballot::{Contest, Weight},
    services::area_tree::TreeNode,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tracing::{debug, event, info, instrument, Level, Value as TracingValue};
use uuid::Uuid;

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

fn load_tally_sheet_results(
    tally_sheets_dir: &Path,
    contest: &Contest,
) -> Result<Vec<(ContestResult, TallySheet)>> {
    if !tally_sheets_dir.is_dir() {
        return Ok(vec![]);
    }

    list_tally_sheet_subfolders(tally_sheets_dir)
        .into_iter()
        .map(|tally_sheet_folder| {
            let tally_sheet_file_path = tally_sheet_folder.join(INPUT_TALLY_SHEET_FILE);
            let tally_sheet_str = fs::read_to_string(&tally_sheet_file_path)
                .map_err(|error| Error::FileAccess(tally_sheet_file_path, error))?;
            let tally_sheet: TallySheet = serde_json::from_str(&tally_sheet_str)?;
            let contest_result = tally::process_tally_sheet(&tally_sheet, contest)
                .map_err(|error| Error::UnexpectedError(error.to_string()))?;
            validate_votes_by_channel(&contest_result)?;
            Ok((contest_result, tally_sheet))
        })
        .collect()
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
                    *current_result = current_result.aggregate(contest_result, true);
                })
                .or_insert_with(|| contest_result.clone());
        }

        for (channel, contest_result) in breakdown_map {
            let breakdown_folder_path = base_breakdown_path.join(&channel.to_string());
            fs::create_dir_all(&breakdown_folder_path)?;
            let breakdown_file_path = breakdown_folder_path.join(OUTPUT_CONTEST_RESULT_FILE);
            debug!("breakdown_file_path: {}", breakdown_file_path.display());
            let contest_result_file = fs::File::create(&breakdown_file_path)?;
            serde_json::to_writer(contest_result_file, &contest_result)?;
        }

        Ok(())
    }
}

fn participation_total(result: &ContestResult) -> Result<u64> {
    let declined = result
        .extended_metrics
        .as_ref()
        .map(|metrics| metrics.total_declined_to_vote)
        .unwrap_or_default();

    result
        .total_votes
        .checked_add(result.auditable_votes)
        .and_then(|total| total.checked_add(declined))
        .ok_or_else(|| Error::UnexpectedError("Participation total overflow".to_string()))
}

fn merge_votes_by_channel(
    aggregate: &mut BTreeMap<String, u64>,
    counts: &BTreeMap<String, u64>,
) -> Result<()> {
    for (channel, count) in counts {
        let current = aggregate.entry(channel.clone()).or_default();
        *current = current.checked_add(*count).ok_or_else(|| {
            Error::UnexpectedError(format!("Voting channel count overflow for {channel}"))
        })?;
    }
    Ok(())
}

fn merge_result_votes_by_channel(
    aggregate: &mut BTreeMap<String, u64>,
    result: &ContestResult,
) -> Result<()> {
    if let Some(metrics) = &result.extended_metrics {
        merge_votes_by_channel(aggregate, &metrics.votes_by_channel)?;
    }
    Ok(())
}

fn aggregate_area_votes_by_channel<'a>(
    area_ids: impl IntoIterator<Item = &'a str>,
    votes_by_channel_map: &HashMap<String, Option<BTreeMap<String, u64>>>,
) -> Result<(BTreeMap<String, u64>, bool)> {
    let mut aggregate = BTreeMap::new();
    let mut all_inputs_present = true;

    for area_id in area_ids {
        // The area tree contains every area in the election, including areas
        // whose ballot style does not contain this contest. Those areas are
        // not inputs for this aggregate and must not make it look incomplete.
        let Some(counts) = votes_by_channel_map.get(area_id) else {
            continue;
        };

        match counts {
            Some(counts) => merge_votes_by_channel(&mut aggregate, counts)?,
            None => all_inputs_present = false,
        }
    }

    Ok((aggregate, all_inputs_present))
}

fn set_votes_by_channel(result: &mut ContestResult, counts: BTreeMap<String, u64>) {
    result
        .extended_metrics
        .get_or_insert_with(ExtendedMetricsContest::default)
        .votes_by_channel = counts;
}

fn validate_votes_by_channel(result: &ContestResult) -> Result<()> {
    let has_counts = result
        .extended_metrics
        .as_ref()
        .is_some_and(|metrics| !metrics.votes_by_channel.is_empty());
    if !has_counts {
        return Ok(());
    }

    validate_complete_votes_by_channel(result)
}

fn validate_complete_votes_by_channel(result: &ContestResult) -> Result<()> {
    let channel_total = result
        .extended_metrics
        .as_ref()
        .map(|metrics| metrics.votes_by_channel.values())
        .into_iter()
        .flatten()
        .try_fold(0u64, |total, count| {
            total
                .checked_add(*count)
                .ok_or_else(|| Error::UnexpectedError("Voting channel total overflow".to_string()))
        })?;
    let participation_total = participation_total(result)?;

    if channel_total != participation_total {
        return Err(Error::UnexpectedError(format!(
            "Voting channel total {channel_total} does not match participation total {participation_total} for contest {}",
            result.contest.id
        )));
    }

    Ok(())
}

fn has_complete_votes_by_channel(result: &ContestResult) -> Result<bool> {
    validate_votes_by_channel(result)?;
    let has_counts = result
        .extended_metrics
        .as_ref()
        .is_some_and(|metrics| !metrics.votes_by_channel.is_empty());
    Ok(participation_total(result)? == 0 || has_counts)
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
                    let contest_object = contest_input.contest.clone();
                    let contest_op = get_contest_tally_operation(&contest_object);

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

                    let areas_tree = Arc::new(
                        TreeNode::<()>::from_areas(election_input.areas.clone()).map_err(
                            |err| {
                                Error::UnexpectedError(format!(
                                    "Error building area tree for contest {}: {:?}",
                                    contest_id_for_contest, err
                                ))
                            },
                        )?,
                    );

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
                    let votes_by_channel_map: HashMap<String, Option<BTreeMap<String, u64>>> =
                        contest_input
                            .area_list
                            .iter()
                            .map(|area_input| {
                                (
                                    area_input.area.id.to_string(),
                                    area_input.area.votes_by_channel.clone(),
                                )
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

                            let area_op = get_area_tally_operation(
                                &election_input.ballot_styles,
                                contest_object.get_counting_algorithm(),
                                &area_input.id,
                            );

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

                                let children_area_paths: Vec<(PathBuf, Weight)> = children_areas
                                    .iter()
                                    .map(|child_area| -> Result<(PathBuf, Weight), Error> {
                                        let child_area_id = Uuid::parse_str(&child_area.id)
                                            .map_err(|err| {
                                                Error::UnexpectedError(format!(
                                                    "Uuid parse error: {err:?}"
                                                ))
                                            })?;

                                        let child_area_weight = get_area_weight(
                                            &election_input.ballot_styles,
                                            &child_area_id,
                                        );

                                        Ok((
                                            PipeInputs::build_path(
                                                &input_dir,
                                                &election_id,
                                                Some(&contest_id),
                                                Some(&child_area_id),
                                            )
                                            .join(OUTPUT_DECODED_BALLOTS_FILE),
                                            child_area_weight,
                                        ))
                                    })
                                    .collect::<Result<Vec<(PathBuf, Weight)>, Error>>()?;

                                let counting_algorithm = tally::create_tally(
                                    &contest_object,
                                    ScopeOperation::Area(area_op), // The operation of the parent area is used in the aggregate of its children, this makes sense so that each child has the same data available
                                    children_area_paths,
                                    census_size,
                                    auditable_votes_size,
                                    vec![],
                                    vec![],
                                )
                                .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                                let mut aggregate_result: ContestResult = counting_algorithm
                                    .tally()
                                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                                let (electronic_channel_counts, all_channel_inputs_present) =
                                    aggregate_area_votes_by_channel(
                                        children_areas.iter().map(|area| area.id.as_str()),
                                        &votes_by_channel_map,
                                    )?;
                                set_votes_by_channel(
                                    &mut aggregate_result,
                                    electronic_channel_counts,
                                );
                                let has_complete_electronic_channels = if all_channel_inputs_present
                                {
                                    validate_complete_votes_by_channel(&aggregate_result)?;
                                    true
                                } else {
                                    participation_total(&aggregate_result)? == 0
                                };

                                let mut aggregate_tally_sheet_results = vec![];
                                for child_area in &children_areas {
                                    let child_area_id =
                                        Uuid::parse_str(&child_area.id).map_err(|error| {
                                            Error::UnexpectedError(format!(
                                                "Uuid parse error: {error:?}"
                                            ))
                                        })?;
                                    let child_tally_sheets_dir = PipeInputs::build_path(
                                        &tally_sheets_dir,
                                        &election_id,
                                        Some(&contest_id),
                                        Some(&child_area_id),
                                    );
                                    aggregate_tally_sheet_results.extend(load_tally_sheet_results(
                                        &child_tally_sheets_dir,
                                        &contest_object,
                                    )?);
                                }

                                aggregate_result = aggregate_tally_sheet_results.iter().fold(
                                    aggregate_result,
                                    |result, (tally_sheet_result, _)| {
                                        result.aggregate(tally_sheet_result, false)
                                    },
                                );
                                if !has_complete_electronic_channels {
                                    set_votes_by_channel(&mut aggregate_result, BTreeMap::new());
                                }
                                validate_votes_by_channel(&aggregate_result)?;

                                let file_path =
                                    base_aggregate_path.join(OUTPUT_CONTEST_RESULT_FILE);
                                let file = fs::File::create(file_path)?;
                                serde_json::to_writer_pretty(file, &aggregate_result)?;
                                // Using pretty for readability
                            }

                            let area_weight =
                                get_area_weight(&election_input.ballot_styles, &area_input.id);

                            // Create area tally
                            let counting_algorithm_area = tally::create_tally(
                                &contest_object,
                                ScopeOperation::Area(area_op),
                                vec![(decoded_ballots_file.clone(), area_weight)],
                                area_input.census,
                                area_input.auditable_votes,
                                vec![],
                                vec![],
                            )
                            .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                            let mut area_tally_results = counting_algorithm_area
                                .tally()
                                .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                            let has_channel_input = area_input.area.votes_by_channel.is_some();
                            {
                                let extended_metrics = area_tally_results
                                    .extended_metrics
                                    .get_or_insert_with(ExtendedMetricsContest::default);
                                extended_metrics.weight = area_weight;
                                extended_metrics.votes_by_channel =
                                    area_input.area.votes_by_channel.clone().unwrap_or_default();
                            }
                            let has_complete_electronic_channels = if has_channel_input {
                                validate_complete_votes_by_channel(&area_tally_results)?;
                                true
                            } else {
                                participation_total(&area_tally_results)? == 0
                            };

                            // Tally sheets tally for this area
                            let input_tally_sheets_dir_path = PipeInputs::build_path(
                                &tally_sheets_dir,
                                &election_id,
                                Some(&contest_id),
                                Some(&area_id),
                            );
                            let area_specific_tally_sheet_results = load_tally_sheet_results(
                                &input_tally_sheets_dir_path,
                                &contest_object,
                            )?;
                            for (contest_result_sheet, tally_sheet) in
                                &area_specific_tally_sheet_results
                            {
                                let output_tally_sheets_folder_path =
                                    PipeInputs::build_tally_sheet_path(
                                        &base_output_path,
                                        &tally_sheet.id,
                                    );
                                fs::create_dir_all(&output_tally_sheets_folder_path)?;
                                let output_tally_sheets_file_path = output_tally_sheets_folder_path
                                    .join(OUTPUT_CONTEST_RESULT_FILE);
                                let contest_result_file_sheet =
                                    fs::File::create(&output_tally_sheets_file_path)?;
                                serde_json::to_writer_pretty(
                                    contest_result_file_sheet,
                                    contest_result_sheet,
                                )?;
                            }

                            let mut area_result_with_tally_sheets =
                                area_specific_tally_sheet_results.iter().fold(
                                    area_tally_results.clone(),
                                    |result, (tally_sheet_result, _)| {
                                        result.aggregate(tally_sheet_result, false)
                                    },
                                );
                            if !has_complete_electronic_channels {
                                set_votes_by_channel(
                                    &mut area_result_with_tally_sheets,
                                    BTreeMap::new(),
                                );
                            }
                            validate_votes_by_channel(&area_result_with_tally_sheets)?;

                            fs::create_dir_all(&base_output_path)?;
                            let file_path_area = base_output_path.join(OUTPUT_CONTEST_RESULT_FILE);
                            let file_area = fs::File::create(file_path_area)?;
                            serde_json::to_writer_pretty(
                                file_area,
                                &area_result_with_tally_sheets,
                            )?;

                            // Return data needed for final aggregation for the contest
                            Ok((
                                (decoded_ballots_file, area_weight),
                                area_input.census,
                                area_input.auditable_votes,
                                area_specific_tally_sheet_results,
                                area_tally_results,
                            ))
                        })
                        .collect(); // Collects Result<Vec<(PathBuf, u64, u64, Vec<_>)>, Error>

                    let collected_area_outputs = area_processing_results?; // Propagate error if any area failed

                    // Aggregate results from parallel area processing
                    let mut contest_ballot_files: Vec<(PathBuf, Weight)> = vec![];
                    let mut sum_census: u64 = 0;
                    let mut sum_auditable_votes: u64 = 0;
                    let mut tally_sheet_results_for_contest: Vec<(ContestResult, TallySheet)> =
                        vec![];
                    let mut area_tally_results_for_contest: Vec<ContestResult> = vec![];

                    for (
                        ballot_file,
                        census,
                        auditable_votes_val,
                        sheet_results,
                        area_tally_results,
                    ) in collected_area_outputs
                    {
                        contest_ballot_files.push(ballot_file);
                        sum_census += census;
                        sum_auditable_votes += auditable_votes_val;
                        tally_sheet_results_for_contest.extend(sheet_results);
                        area_tally_results_for_contest.push(area_tally_results);
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
                    let has_complete_electronic_channels = area_tally_results_for_contest
                        .iter()
                        .try_fold(true, |is_complete, area_result| {
                            Ok::<bool, Error>(
                                is_complete && has_complete_votes_by_channel(area_result)?,
                            )
                        })?;
                    let mut final_channel_counts = BTreeMap::new();
                    if has_complete_electronic_channels {
                        for result in area_tally_results_for_contest
                            .iter()
                            .chain(final_only_sheet_results.iter())
                        {
                            merge_result_votes_by_channel(&mut final_channel_counts, result)?;
                        }
                    }

                    // Create final contest tally
                    let final_counting_algorithm = tally::create_tally(
                        &contest_object,
                        ScopeOperation::Contest(contest_op),
                        contest_ballot_files,
                        sum_census,
                        sum_auditable_votes,
                        final_only_sheet_results,
                        area_tally_results_for_contest,
                    )
                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                    let mut final_res = final_counting_algorithm
                        .tally()
                        .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                    set_votes_by_channel(&mut final_res, final_channel_counts);
                    validate_votes_by_channel(&final_res)?;

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

/// A counter of ballots split by whether the voter expressed the condition
/// explicitly (e.g. by selecting a marker candidate) or implicitly.
///
/// Used for both blank and invalid vote counts; the serialized field names
/// are shared by both usages.
#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
pub struct ExplicitImplicitCount {
    pub explicit: u64,
    pub implicit: u64,
}

impl ExplicitImplicitCount {
    pub fn new(explicit: u64, implicit: u64) -> Self {
        ExplicitImplicitCount { explicit, implicit }
    }

    pub fn aggregate(&self, other: &ExplicitImplicitCount) -> ExplicitImplicitCount {
        let mut sum = *self;

        sum.explicit += other.explicit;
        sum.implicit += other.implicit;
        sum
    }

    pub fn total(&self) -> u64 {
        self.explicit + self.implicit
    }
}

/// Invalid vote counts, kept as a type distinct from [`BlankVotes`].
#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
#[serde(transparent)]
pub struct InvalidVotes(pub ExplicitImplicitCount);

impl InvalidVotes {
    pub fn new(explicit: u64, implicit: u64) -> Self {
        InvalidVotes(ExplicitImplicitCount::new(explicit, implicit))
    }

    #[instrument]
    pub fn aggregate(&self, other: &InvalidVotes) -> InvalidVotes {
        InvalidVotes(self.0.aggregate(&other.0))
    }
}

impl Deref for InvalidVotes {
    type Target = ExplicitImplicitCount;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InvalidVotes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Blank vote counts, kept as a type distinct from [`InvalidVotes`].
#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
#[serde(transparent)]
pub struct BlankVotes(pub ExplicitImplicitCount);

impl BlankVotes {
    pub fn new(explicit: u64, implicit: u64) -> Self {
        BlankVotes(ExplicitImplicitCount::new(explicit, implicit))
    }

    #[instrument]
    pub fn aggregate(&self, other: &BlankVotes) -> BlankVotes {
        BlankVotes(self.0.aggregate(&other.0))
    }
}

impl Deref for BlankVotes {
    type Target = ExplicitImplicitCount;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BlankVotes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
    pub weight: Weight, // Used to store the actual weight used to tally an specific area.
    pub total_weight: u64, // Used to calculate the right percentage_votes in aggregate
    pub total_declined_to_vote: u64, // Total number of ballots that declined to vote
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub votes_by_channel: BTreeMap<String, u64>,
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
        result.total_weight += other.total_weight;
        result.total_declined_to_vote += other.total_declined_to_vote;
        for (channel, count) in &other.votes_by_channel {
            *result.votes_by_channel.entry(channel.clone()).or_default() += count;
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedMetricsElection {
    // Number of valid ballots processed by the ACM without any
    // single mark on all contests.
    pub abstentions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContestResult {
    pub contest: Contest,
    pub census: u64,
    pub percentage_census: f64,
    pub auditable_votes: u64,
    pub percentage_auditable_votes: f64,
    pub total_votes: u64,
    pub percentage_total_votes: f64,
    /// Ballots that are not invalid and not declined. Explicit and implicit
    /// blank ballots are included; selecting explicit blank with a regular
    /// candidate is an implicit invalid ballot.
    pub total_valid_votes: u64,
    pub percentage_total_valid_votes: f64,
    pub total_invalid_votes: u64,
    pub percentage_total_invalid_votes: f64,
    pub total_blank_votes: u64,
    pub percentage_total_blank_votes: f64,
    pub blank_votes: BlankVotes,
    pub invalid_votes: InvalidVotes,
    pub percentage_blank_votes_explicit: f64,
    pub percentage_blank_votes_implicit: f64,
    pub percentage_invalid_votes_explicit: f64,
    pub percentage_invalid_votes_implicit: f64,
    pub candidate_result: Vec<CandidateResult>,
    pub extended_metrics: Option<ExtendedMetricsContest>,
    pub process_results: Option<Value>, // The results from the counting algorithm process
}

impl ContestResult {
    #[instrument(skip_all)]
    pub fn calculate_percentages(&self) -> ContestResult {
        let extended_metrics = self.extended_metrics.clone().unwrap_or_default();
        let total_weight = extended_metrics.total_weight;
        let candidate_votes_base = if total_weight > 0 {
            total_weight
        } else {
            self.total_valid_votes
                .saturating_sub(self.total_blank_votes)
        };
        let explicit_vote_base = if extended_metrics.total_ballots > 0 {
            extended_metrics.total_ballots
        } else {
            self.total_votes
        };
        let candidate_result: Vec<CandidateResult> = self
            .candidate_result
            .clone()
            .into_iter()
            .map(|candidate_result| {
                let percentage_votes = if candidate_result.candidate.is_explicit_blank() {
                    (self.blank_votes.explicit as f64 / cmp::max(1, explicit_vote_base) as f64)
                        * 100.0
                } else if candidate_result.candidate.is_explicit_invalid() {
                    (self.invalid_votes.explicit as f64 / cmp::max(1, explicit_vote_base) as f64)
                        * 100.0
                } else {
                    (candidate_result.total_count as f64 / cmp::max(1, candidate_votes_base) as f64)
                        * 100.0
                };
                let mut new_candidate_result = candidate_result.clone();
                new_candidate_result.percentage_votes = percentage_votes.clamp(0.0, 100.0);

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
        let percentage_blank_votes_explicit =
            (self.blank_votes.explicit as f64 * 100.0) / total_votes_base;
        let percentage_blank_votes_implicit =
            (self.blank_votes.implicit as f64 * 100.0) / total_votes_base;
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
        contest_result.percentage_blank_votes_explicit =
            percentage_blank_votes_explicit.clamp(0.0, 100.0);
        contest_result.percentage_blank_votes_implicit =
            percentage_blank_votes_implicit.clamp(0.0, 100.0);
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
        let aggregate_metrics = aggregate.extended_metrics.take().unwrap_or_default();
        aggregate.extended_metrics =
            Some(aggregate_metrics.aggregate(&other.extended_metrics.clone().unwrap_or_default()));
        aggregate.auditable_votes += other.auditable_votes;
        aggregate.total_votes += other.total_votes;
        aggregate.total_valid_votes += other.total_valid_votes;
        aggregate.total_invalid_votes += other.total_invalid_votes;
        aggregate.total_blank_votes += other.total_blank_votes;
        aggregate.blank_votes = aggregate.blank_votes.aggregate(&other.blank_votes);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_metrics_aggregate_channel_counts() {
        let left = ExtendedMetricsContest {
            votes_by_channel: BTreeMap::from([
                ("ONLINE".to_string(), 3),
                ("TELEPHONE".to_string(), 1),
            ]),
            ..Default::default()
        };
        let right = ExtendedMetricsContest {
            votes_by_channel: BTreeMap::from([("ONLINE".to_string(), 2), ("PAPER".to_string(), 4)]),
            ..Default::default()
        };

        let aggregate = left.aggregate(&right);

        assert_eq!(aggregate.votes_by_channel.get("ONLINE"), Some(&5));
        assert_eq!(aggregate.votes_by_channel.get("TELEPHONE"), Some(&1));
        assert_eq!(aggregate.votes_by_channel.get("PAPER"), Some(&4));
    }

    #[test]
    fn contest_result_aggregation_preserves_auditable_channel_participation() {
        let area_result = ContestResult {
            census: 1,
            total_votes: 1,
            auditable_votes: 1,
            extended_metrics: Some(ExtendedMetricsContest {
                votes_by_channel: BTreeMap::from([("ONLINE".to_string(), 2)]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let aggregate = ContestResult::default().aggregate(&area_result, true);

        assert_eq!(aggregate.auditable_votes, 1);
        assert!(validate_votes_by_channel(&aggregate).is_ok());
    }

    #[test]
    fn area_channel_aggregation_ignores_areas_outside_the_contest() {
        let area_ids = ["parent", "contest-child", "other-contest-child"];
        let votes_by_channel_map = HashMap::from([
            (
                "parent".to_string(),
                Some(BTreeMap::from([("ONLINE".to_string(), 2)])),
            ),
            (
                "contest-child".to_string(),
                Some(BTreeMap::from([("TELEPHONE".to_string(), 1)])),
            ),
        ]);

        let (aggregate, all_inputs_present) =
            aggregate_area_votes_by_channel(area_ids.iter().copied(), &votes_by_channel_map)
                .unwrap();

        assert!(all_inputs_present);
        assert_eq!(aggregate.get("ONLINE"), Some(&2));
        assert_eq!(aggregate.get("TELEPHONE"), Some(&1));
    }

    #[test]
    fn area_channel_aggregation_detects_a_legacy_contest_area() {
        let votes_by_channel_map = HashMap::from([
            (
                "parent".to_string(),
                Some(BTreeMap::from([("ONLINE".to_string(), 2)])),
            ),
            ("legacy-child".to_string(), None),
        ]);

        let (_, all_inputs_present) =
            aggregate_area_votes_by_channel(["parent", "legacy-child"], &votes_by_channel_map)
                .unwrap();

        assert!(!all_inputs_present);
    }

    #[test]
    fn channel_validation_includes_auditable_and_declined_ballots() {
        let result = ContestResult {
            total_votes: 4,
            auditable_votes: 1,
            extended_metrics: Some(ExtendedMetricsContest {
                total_declined_to_vote: 1,
                votes_by_channel: BTreeMap::from([("ONLINE".to_string(), 6)]),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(validate_votes_by_channel(&result).is_ok());
    }

    #[test]
    fn legacy_participation_without_channel_counts_is_incomplete_but_valid() {
        let result = ContestResult {
            total_votes: 2,
            extended_metrics: Some(ExtendedMetricsContest::default()),
            ..Default::default()
        };

        assert!(validate_votes_by_channel(&result).is_ok());
        assert!(!has_complete_votes_by_channel(&result).unwrap());
    }

    #[test]
    fn channel_validation_rejects_a_partial_non_empty_breakdown() {
        let result = ContestResult {
            total_votes: 2,
            extended_metrics: Some(ExtendedMetricsContest {
                votes_by_channel: BTreeMap::from([("ONLINE".to_string(), 1)]),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(validate_votes_by_channel(&result).is_err());
    }
}
